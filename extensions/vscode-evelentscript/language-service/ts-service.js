'use strict';

const fs = require('fs');
const path = require('path');
const ts = require('typescript');
const { compileToVirtualTypeScript, virtualTsPath, getSyntaxError } = require('./compile-virtual');
const { mapToGenerated, mapManyToOriginal } = require('./mapping');
const { getFallbackCompletions, isMemberAccessContext, isKeywordEntry, isInString, isModuleSpecifierContext } = require('./global-completions');
const { getKeywordCompletions } = require('./keywords');

// Type-compatibility diagnostics that are noise in a dynamically typed language
// (they require type annotations to satisfy). Suppressed unless strict typing
// is explicitly enabled. High-value resolution/typo diagnostics (cannot find
// name/module, property does not exist, "did you mean", argument count) are
// intentionally kept.
const RELAXED_SUPPRESSED_CODES = new Set([
  2322, // Type X is not assignable to type Y
  2339, // Property does not exist on type (false positive for @prop pattern)
  2345, // Argument of type X is not assignable to parameter of type Y
  2551, // Property does not exist on type. Did you mean ...? (same as 2339 with suggestion)
  2769, // No overload matches this call
  2571, // Object is of type 'unknown'
  18046, // 'x' is of type 'unknown'
  18047, // 'x' is possibly 'null'
  18048, // 'x' is possibly 'undefined'
  2531, // Object is possibly 'null'
  2532, // Object is possibly 'undefined'
  2533, // Object is possibly 'null' or 'undefined'
  2356, // An arithmetic operand must be of type ...
  2362, // The left-hand side of an arithmetic operation must be ...
  2363, // The right-hand side of an arithmetic operation must be ...
  2365, // Operator cannot be applied to types
  2349, // This expression is not callable
  2351, // This expression is not constructable
  2488, // Type must have a '[Symbol.iterator]()' method
  2495, // Type is not an array type or a string type
  2538, // Type cannot be used as an index type
  2683, // 'this' implicitly has type 'any'
  2721, // Cannot invoke an object which is possibly 'null'
  2722, // Cannot invoke an object which is possibly 'undefined'
  7053, // Element implicitly has an 'any' type because expression of type ... can't index
]);

class EvelentLanguageService {
  constructor(workspaceRoot, extensionRoot) {
    this.workspaceRoot = workspaceRoot;
    this.extensionRoot = extensionRoot;
    /** @type {Map<string, { content: string, version: number, sourceMap: object|null }>} */
    this.documents = new Map();
    this.projectVersion = 0;
    // Shared registry lets multiple project services reuse parsed files.
    this.registry = ts.createDocumentRegistry();
    this.defaultOptions = this.buildDefaultOptions();
    /** @type {Map<string, { service: import('typescript').LanguageService, options: object, mtime: number }>} */
    this.projects = new Map();
    /** @type {Map<string, { options: object, mtime: number }>} */
    this.configCache = new Map();
  }

  buildDefaultOptions() {
    const typeRoots = [
      path.join(this.workspaceRoot, 'node_modules/@types'),
      path.join(this.extensionRoot, 'node_modules/@types'),
    ].filter((dir) => fs.existsSync(dir));

    return {
      ...ts.getDefaultCompilerOptions(),
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.CommonJS,
      lib: ['lib.es2020.d.ts', 'lib.dom.d.ts'],
      allowJs: true,
      checkJs: true,
      skipLibCheck: true,
      esModuleInterop: true,
      // EvelentScript has no parameter/return type syntax in everyday code, so
      // "implicit any" would fire on nearly every function. Keep it off.
      noImplicitAny: false,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      typeRoots: typeRoots.length ? typeRoots : undefined,
      types: ['node'],
      noEmit: true,
    };
  }

  createService(options, currentDir) {
    const self = this;
    const cwd = currentDir || this.workspaceRoot;

    const host = {
      getCompilationSettings: () => options,
      getScriptFileNames: () => [...this.documents.keys()],
      getScriptVersion: (fileName) => {
        const key = self.toPath(fileName);
        return String(self.documents.get(key)?.version ?? 0);
      },
      getScriptSnapshot: (fileName) => {
        const key = self.toPath(fileName);
        const text = self.documents.get(key)?.content;
        if (text != null) {
          return ts.ScriptSnapshot.fromString(text);
        }
        // Fall back to disk so the default lib, @types and node_modules
        // declaration files can be loaded by the language service.
        if (ts.sys.fileExists(fileName)) {
          const fileText = ts.sys.readFile(fileName);
          if (fileText != null) {
            return ts.ScriptSnapshot.fromString(fileText);
          }
        }
        return undefined;
      },
      getProjectVersion: () => String(self.projectVersion),
      getScriptKind: () => ts.ScriptKind.TS,
      getCurrentDirectory: () => cwd,
      getDefaultLibFileName: (opts) => ts.getDefaultLibFilePath(opts),
      fileExists: ts.sys.fileExists,
      readFile: ts.sys.readFile,
      readDirectory: ts.sys.readDirectory,
      directoryExists: ts.sys.directoryExists,
      getCanonicalFileName: (fileName) => self.getCanonicalFileName(fileName),
      useCaseSensitiveFileNames: () => ts.sys.useCaseSensitiveFileNames,
      getNewLine: () => ts.sys.newLine,
      resolveModuleNames: (moduleNames, containingFile) =>
        moduleNames.map((name) => {
          const resolved = ts.resolveModuleName(name, containingFile, options, ts.sys);
          if (resolved.resolvedModule) {
            return resolved.resolvedModule;
          }
          // Fall back to resolving sibling EvelentScript files so relative
          // imports between .es modules get types and completions.
          return self.resolveEsModule(name, containingFile);
        }),
    };

    return ts.createLanguageService(host, this.registry);
  }

  /**
   * Find the nearest tsconfig.json / jsconfig.json by walking up from the file
   * directory toward the filesystem root. Returns an absolute path or null.
   */
  findConfig(esPath) {
    let dir = path.dirname(path.resolve(esPath));
    for (let depth = 0; depth < 64; depth++) {
      for (const name of ['tsconfig.json', 'jsconfig.json']) {
        const candidate = path.join(dir, name);
        if (ts.sys.fileExists(candidate)) {
          return candidate;
        }
      }
      const parent = path.dirname(dir);
      if (parent === dir) {
        break;
      }
      dir = parent;
    }
    return null;
  }

  safeMtime(filePath) {
    try {
      return fs.statSync(filePath).mtimeMs;
    } catch (_) {
      return 0;
    }
  }

  /**
   * Parse a tsconfig/jsconfig and merge it over the default options, while
   * forcing the flags the virtual TS service relies on.
   */
  loadConfigOptions(configPath, mtime) {
    const cached = this.configCache.get(configPath);
    if (cached && cached.mtime === mtime) {
      return cached.options;
    }
    let merged = this.defaultOptions;
    try {
      const read = ts.readConfigFile(configPath, ts.sys.readFile);
      const parsed = ts.parseJsonConfigFileContent(
        read.config || {},
        ts.sys,
        path.dirname(configPath),
        undefined,
        configPath
      );
      merged = {
        ...this.defaultOptions,
        ...parsed.options,
        allowJs: true,
        checkJs: true,
        noEmit: true,
        skipLibCheck: true,
        // Force implicit-any off even if the config enables `strict`, since
        // EvelentScript code rarely carries type annotations.
        noImplicitAny: false,
      };
      // Only keep typeRoots the config explicitly declares. Inheriting the
      // default @types-only roots would stop scoped type packages such as
      // @altv/types-server (which live in node_modules) from resolving.
      merged.typeRoots = parsed.options.typeRoots;
    } catch (_) {
      merged = this.defaultOptions;
    }
    this.configCache.set(configPath, { options: merged, mtime });
    return merged;
  }

  /**
   * Return the TypeScript language service whose compiler options apply to the
   * given .es file (based on the nearest tsconfig/jsconfig).
   */
  serviceFor(esPath) {
    const configPath = this.findConfig(esPath);
    const key = configPath ? this.getCanonicalFileName(configPath) : '__default__';
    const mtime = configPath ? this.safeMtime(configPath) : 0;
    let project = this.projects.get(key);
    if (!project || project.mtime !== mtime) {
      const options = configPath ? this.loadConfigOptions(configPath, mtime) : this.defaultOptions;
      const currentDir = configPath ? path.dirname(configPath) : this.workspaceRoot;
      project = { service: this.createService(options, currentDir), options, mtime };
      this.projects.set(key, project);
    }
    return project.service;
  }

  /**
   * Resolve a relative import to a sibling EvelentScript file's virtual TS.
   * Returns a TypeScript ResolvedModule or undefined when no .es file matches.
   */
  resolveEsModule(name, containingFile) {
    if (!containingFile.endsWith('.evelent.ts') || !name.startsWith('.')) {
      return undefined;
    }
    const esContaining = containingFile.slice(0, -'.evelent.ts'.length);
    const baseDir = path.dirname(esContaining);
    const target = path.resolve(baseDir, name);
    const candidates = [
      `${target}.es`,
      `${target}.lites`,
      `${target}.es.md`,
      path.join(target, 'index.es'),
    ];
    for (const esPath of candidates) {
      if (ts.sys.fileExists(esPath) && this.ensureExternalDocument(esPath)) {
        return {
          resolvedFileName: virtualTsPath(esPath),
          extension: ts.Extension.Ts,
          isExternalLibraryImport: false,
        };
      }
    }
    return undefined;
  }

  /**
   * Lazily compile an .es file that isn't open in the editor so cross-file
   * imports resolve. Reads from disk and refreshes when the file changes.
   */
  ensureExternalDocument(esPath) {
    const tsPath = this.toPath(virtualTsPath(esPath));
    let mtime = 0;
    try {
      mtime = fs.statSync(esPath).mtimeMs;
    } catch (_) {
      // ignore; handled below
    }
    const existing = this.documents.get(tsPath);
    if (existing && existing.external && existing.mtime === mtime) {
      return tsPath;
    }
    // Don't clobber a document that is open and managed by the editor.
    if (existing && !existing.external) {
      return tsPath;
    }
    let source;
    try {
      source = fs.readFileSync(esPath, 'utf8');
    } catch (_) {
      return null;
    }
    let content;
    let v3SourceMap = null;
    let bodyStartLine = 1;
    let strippedLines = 0;
    try {
      ({ content, v3SourceMap, bodyStartLine, strippedLines } = compileToVirtualTypeScript(
        source,
        esPath,
        this.workspaceRoot
      ));
    } catch (_) {
      content = [
        '/// <reference lib="es2020" />',
        '/// <reference types="node" />',
        '',
      ].join('\n');
    }
    this.documents.set(tsPath, {
      content,
      version: existing ? existing.version + 1 : 1,
      sourceMap: v3SourceMap,
      esPath,
      bodyStartLine,
      strippedLines,
      compileError: null,
      external: true,
      mtime,
    });
    this.projectVersion += 1;
    return tsPath;
  }

  updateDocument(esPath, source) {
    let content;
    let v3SourceMap = null;
    let bodyStartLine = 1;
    let strippedLines = 0;
    let compileError = null;
    try {
      ({ content, v3SourceMap, bodyStartLine, strippedLines } = compileToVirtualTypeScript(
        source,
        esPath,
        this.workspaceRoot
      ));
    } catch (error) {
      compileError = this.normalizeCompileError(error, source);
      content = [
        '/// <reference lib="es2020" />',
        '/// <reference types="node" />',
        '',
      ].join('\n');
    }
    // Detect genuine syntax errors on the raw source. Padding lets the virtual
    // TS compile for completions, but it can hide real bracket/indent mistakes.
    if (!compileError) {
      let syntaxError = null;
      try {
        syntaxError = getSyntaxError(source, esPath, this.workspaceRoot);
      } catch (_) {
        syntaxError = null;
      }
      if (syntaxError) {
        compileError = this.normalizeCompileError(syntaxError, source);
      }
    }
    const tsPath = this.toPath(virtualTsPath(esPath));
    const existing = this.documents.get(tsPath);
    const version = existing ? existing.version + 1 : 1;
    this.documents.set(tsPath, {
      content,
      version,
      sourceMap: v3SourceMap,
      esPath,
      bodyStartLine,
      strippedLines,
      compileError,
    });
    this.projectVersion += 1;
    return tsPath;
  }

  /**
   * Normalize a compiler SyntaxError into a diagnostic-friendly object with a
   * zero-based range in the original .es source.
   */
  normalizeCompileError(error, source) {
    const loc = error && error.location;
    let startLine = 0;
    let startColumn = 0;
    let endLine = 0;
    let endColumn = 1;
    if (loc) {
      startLine = loc.first_line ?? 0;
      startColumn = loc.first_column ?? 0;
      endLine = loc.last_line ?? startLine;
      endColumn = (loc.last_column ?? startColumn) + 1;
    } else {
      // No location info: highlight the first non-empty line.
      const lines = String(source).split('\n');
      const idx = lines.findIndex((line) => line.trim().length > 0);
      startLine = endLine = idx < 0 ? 0 : idx;
      endColumn = (lines[startLine] || '').length || 1;
    }
    return {
      message: error?.message || 'Syntax error',
      startLine,
      startColumn,
      endLine,
      endColumn,
    };
  }

  removeDocument(esPath) {
    this.documents.delete(this.toPath(virtualTsPath(esPath)));
  }

  getCanonicalFileName(fileName) {
    return ts.sys.useCaseSensitiveFileNames ? fileName : fileName.toLowerCase();
  }

  toPath(filePath) {
    return ts.toPath(path.resolve(filePath), this.workspaceRoot, (f) => this.getCanonicalFileName(f));
  }

  tsPathForEs(esPath) {
    return this.toPath(virtualTsPath(esPath));
  }

  async getGeneratedPosition(esPath, line, column) {
    const tsPath = this.tsPathForEs(esPath);
    const doc = this.documents.get(tsPath);
    if (!doc) {
      return { line, column };
    }
    const generated = await mapToGenerated(doc.sourceMap, esPath, line, column);
    if (!doc.sourceMap) {
      return generated;
    }
    // Convert generated JS line into the virtual TS content line by accounting
    // for the prepended reference/type header and any stripped leading comment.
    return {
      line: generated.line + (doc.bodyStartLine ?? 1) - 1 - (doc.strippedLines ?? 0),
      column: generated.column,
    };
  }

  async getCompletions(esPath, line, column, sourceLine = '') {
    const memberAccess = isMemberAccessContext(sourceLine, column);
    const inString = isInString(sourceLine, column);
    const tsPath = this.tsPathForEs(esPath);

    // Inside an import/require module specifier: list available modules. We
    // locate the string offset directly in the generated TS because source
    // maps don't reliably place positions inside string literals.
    if (inString && isModuleSpecifierContext(sourceLine, column) && this.documents.has(tsPath)) {
      const moduleEntries = this.getModuleSpecifierCompletions(esPath, tsPath, sourceLine, column);
      return moduleEntries?.length ? { entries: moduleEntries } : undefined;
    }

    // Inside any other string literal (e.g. a function argument), language
    // keywords and global identifiers are noise — offer nothing.
    if (inString) {
      return undefined;
    }

    const entries = [];
    const names = new Set();

    const addEntry = (entry) => {
      if (!names.has(entry.name)) {
        names.add(entry.name);
        entries.push(entry);
      }
    };

    if (!memberAccess) {
      for (const item of getKeywordCompletions(sourceLine, column)) {
        addEntry({
          name: item.name,
          kind: 'keyword',
          sortText: `!00${item.name}`,
        });
      }
    }

    if (this.documents.has(tsPath)) {
      const generated = await this.getGeneratedPosition(esPath, line, column);
      const offset = this.offsetAt(tsPath, generated.line, generated.column);
      const options = memberAccess ? { triggerCharacter: '.' } : undefined;

      const service = this.serviceFor(esPath);
      let result = service.getCompletionsAtPosition(tsPath, offset, options);

      if (memberAccess && (!result?.entries?.length || result.entries.every((e) => isKeywordEntry(e.name)))) {
        const dotOffset = Math.max(0, offset - 1);
        result = service.getCompletionsAtPosition(tsPath, dotOffset, { triggerCharacter: '.' }) || result;
      }

      for (const entry of result?.entries || []) {
        if (memberAccess && isKeywordEntry(entry.name)) {
          continue;
        }
        addEntry({
          ...entry,
          sortText: entry.sortText || `!1${entry.name}`,
        });
      }
    }

    for (const item of getFallbackCompletions(sourceLine, column)) {
      addEntry({
        name: item.name,
        kind: item.kind === 'module' ? 'external module name' : 'property',
        sortText: `!0${item.name}`,
      });
    }

    if (!entries.length) {
      return undefined;
    }

    return { entries };
  }

  /**
   * Completions for an import/require module specifier. Finds the string offset
   * directly in the generated TS (source maps are unreliable inside strings).
   */
  getModuleSpecifierCompletions(esPath, tsPath, sourceLine, column) {
    const content = this.documents.get(tsPath)?.content;
    if (!content) {
      return undefined;
    }
    const before = sourceLine.slice(0, column);
    const match = before.match(/(['"])([^'"]*)$/);
    if (!match) {
      return undefined;
    }
    const quote = match[1];
    const partial = match[2];
    // Locate the same opening-quote + partial inside the generated module
    // specifier. Any matching import yields the same module list, so the first
    // occurrence is fine.
    const needle = `from ${quote}${partial}`;
    let idx = content.indexOf(needle);
    let offset;
    if (idx >= 0) {
      offset = idx + needle.length;
    } else {
      const alt = `${quote}${partial}`;
      idx = content.indexOf(`import ${alt}`);
      if (idx >= 0) {
        offset = idx + `import ${quote}`.length + partial.length;
      } else {
        return undefined;
      }
    }
    const result = this.serviceFor(esPath).getCompletionsAtPosition(tsPath, offset, {});
    if (!result?.entries?.length) {
      return undefined;
    }
    return result.entries.map((entry) => ({
      name: entry.name,
      kind: entry.kind,
      sortText: entry.sortText || `!0${entry.name}`,
    }));
  }

  async getCompletionDetails(esPath, line, column, name, source) {
    const tsPath = this.tsPathForEs(esPath);
    const generated = await this.getGeneratedPosition(esPath, line, column);
    return this.serviceFor(esPath).getCompletionEntryDetails(
      tsPath,
      this.offsetAt(tsPath, generated.line, generated.column),
      name,
      undefined,
      undefined,
      undefined,
      source
    );
  }

  async getQuickInfo(esPath, line, column) {
    const tsPath = this.tsPathForEs(esPath);
    if (!this.documents.has(tsPath)) {
      return undefined;
    }
    const generated = await this.getGeneratedPosition(esPath, line, column);
    return this.serviceFor(esPath).getQuickInfoAtPosition(
      tsPath,
      this.offsetAt(tsPath, generated.line, generated.column)
    );
  }

  async getDefinition(esPath, line, column) {
    const tsPath = this.tsPathForEs(esPath);
    if (!this.documents.has(tsPath)) {
      return undefined;
    }
    const generated = await this.getGeneratedPosition(esPath, line, column);
    return this.serviceFor(esPath).getDefinitionAndBoundSpan(
      tsPath,
      this.offsetAt(tsPath, generated.line, generated.column)
    );
  }

  async getSignatureHelp(esPath, line, column) {
    const tsPath = this.tsPathForEs(esPath);
    if (!this.documents.has(tsPath)) {
      return undefined;
    }
    const generated = await this.getGeneratedPosition(esPath, line, column);
    return this.serviceFor(esPath).getSignatureHelpItems(
      tsPath,
      this.offsetAt(tsPath, generated.line, generated.column),
      undefined
    );
  }

  /**
   * Produce diagnostics for an .es file in zero-based original coordinates.
   * Returns compiler syntax errors directly, otherwise TypeScript syntactic and
   * semantic diagnostics mapped back from the virtual TS file.
   *
   * @param {string} esPath
   * @param {{ semantic?: boolean }} [options]
   * @returns {Promise<Array<{ startLine: number, startColumn: number, endLine: number, endColumn: number, message: string, severity: 'error'|'warning' }>>}
   */
  async getDiagnostics(esPath, options = {}) {
    const tsPath = this.tsPathForEs(esPath);
    const doc = this.documents.get(tsPath);
    if (!doc) {
      return [];
    }

    // A compiler error means we have no usable virtual TS; report it alone so
    // typos and syntax mistakes surface immediately at the right spot.
    if (doc.compileError) {
      const e = doc.compileError;
      return [
        {
          startLine: e.startLine,
          startColumn: e.startColumn,
          endLine: e.endLine,
          endColumn: e.endColumn,
          message: e.message,
          severity: 'error',
        },
      ];
    }

    const tsDiagnostics = [
      ...this.serviceFor(esPath).getSyntacticDiagnostics(tsPath),
      ...(options.semantic === false ? [] : this.serviceFor(esPath).getSemanticDiagnostics(tsPath)),
    ].filter((diag) => options.strictTypes || !RELAXED_SUPPRESSED_CODES.has(diag.code));
    if (!tsDiagnostics.length) {
      return [];
    }

    // Collect the start/end positions (in generated JS coordinates) to map in a
    // single batch through the source map.
    const bodyStartLine = doc.bodyStartLine ?? 1;
    const strippedLines = doc.strippedLines ?? 0;
    const positions = [];
    const meta = [];

    for (const diag of tsDiagnostics) {
      if (typeof diag.start !== 'number') {
        continue;
      }
      const startPos = this.lineColFromOffset(doc.content, diag.start);
      const endPos = this.lineColFromOffset(doc.content, diag.start + (diag.length || 0));
      // Skip diagnostics that point into the generated-only header.
      if (startPos.line < bodyStartLine) {
        continue;
      }
      const toJs = (p) => ({
        line: p.line - bodyStartLine + 1 + strippedLines,
        column: p.column,
      });
      positions.push(toJs(startPos), toJs(endPos));
      meta.push({ diag });
    }

    if (!meta.length) {
      return [];
    }

    const mapped = await mapManyToOriginal(doc.sourceMap, positions);
    const diagnostics = [];

    for (let i = 0; i < meta.length; i++) {
      const start = mapped[i * 2];
      const end = mapped[i * 2 + 1];
      if (!start) {
        continue;
      }
      const startLine = start.line - 1;
      const startColumn = start.column;
      let endLine = end ? end.line - 1 : startLine;
      let endColumn = end ? end.column : startColumn + 1;
      if (endLine < startLine || (endLine === startLine && endColumn <= startColumn)) {
        endLine = startLine;
        endColumn = startColumn + 1;
      }
      const diag = meta[i].diag;
      diagnostics.push({
        startLine,
        startColumn,
        endLine,
        endColumn,
        message: ts.flattenDiagnosticMessageText(diag.messageText, '\n').replace(/\.es\.evelent\b/g, '.es'),
        severity: diag.category === ts.DiagnosticCategory.Warning ? 'warning' : 'error',
      });
    }

    return diagnostics;
  }

  /**
   * Convert a zero-based offset in content into a 1-based line and 0-based column.
   */
  lineColFromOffset(content, offset) {
    let line = 1;
    let column = 0;
    const max = Math.min(offset, content.length);
    for (let i = 0; i < max; i++) {
      if (content[i] === '\n') {
        line++;
        column = 0;
      } else {
        column++;
      }
    }
    return { line, column };
  }

  offsetAt(fileName, line, column) {
    const content = this.documents.get(fileName)?.content ?? '';
    const lines = content.split('\n');
    let offset = 0;
    for (let i = 0; i < line - 1 && i < lines.length; i++) {
      offset += lines[i].length + 1;
    }
    offset += column;
    return offset;
  }
}

module.exports = {
  EvelentLanguageService,
};
