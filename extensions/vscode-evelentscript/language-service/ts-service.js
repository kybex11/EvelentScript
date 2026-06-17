'use strict';

const fs = require('fs');
const path = require('path');
const ts = require('typescript');
const { compileToVirtualTypeScript, virtualTsPath, getSyntaxError } = require('./compile-virtual');
const { mapToGenerated, mapManyToOriginal } = require('./mapping');
const { getFallbackCompletions, isMemberAccessContext, isKeywordEntry } = require('./global-completions');
const { getKeywordCompletions } = require('./keywords');

class EvelentLanguageService {
  constructor(workspaceRoot, extensionRoot) {
    this.workspaceRoot = workspaceRoot;
    this.extensionRoot = extensionRoot;
    /** @type {Map<string, { content: string, version: number, sourceMap: object|null }>} */
    this.documents = new Map();
    this.projectVersion = 0;
    this.service = this.createService();
  }

  createService() {
    const typeRoots = [
      path.join(this.workspaceRoot, 'node_modules/@types'),
      path.join(this.extensionRoot, 'node_modules/@types'),
    ].filter((dir) => fs.existsSync(dir));

    this.compilerOptions = {
      ...ts.getDefaultCompilerOptions(),
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.CommonJS,
      lib: ['lib.es2020.d.ts', 'lib.dom.d.ts'],
      allowJs: true,
      checkJs: true,
      skipLibCheck: true,
      esModuleInterop: true,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      typeRoots: typeRoots.length ? typeRoots : undefined,
      types: ['node'],
      noEmit: true,
    };

    const self = this;

    const host = {
      getCompilationSettings: () => this.compilerOptions,
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
      getCurrentDirectory: () => this.workspaceRoot,
      getDefaultLibFileName: (options) => ts.getDefaultLibFilePath(options),
      fileExists: ts.sys.fileExists,
      readFile: ts.sys.readFile,
      readDirectory: ts.sys.readDirectory,
      directoryExists: ts.sys.directoryExists,
      getCanonicalFileName: (fileName) => self.getCanonicalFileName(fileName),
      useCaseSensitiveFileNames: () => ts.sys.useCaseSensitiveFileNames,
      getNewLine: () => ts.sys.newLine,
      resolveModuleNames: (moduleNames, containingFile) =>
        moduleNames.map((name) => {
          const resolved = ts.resolveModuleName(name, containingFile, this.compilerOptions, ts.sys);
          return resolved.resolvedModule;
        }),
    };

    return ts.createLanguageService(host, ts.createDocumentRegistry());
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

    const tsPath = this.tsPathForEs(esPath);
    if (this.documents.has(tsPath)) {
      const generated = await this.getGeneratedPosition(esPath, line, column);
      const offset = this.offsetAt(tsPath, generated.line, generated.column);
      const options = memberAccess ? { triggerCharacter: '.' } : undefined;

      let result = this.service.getCompletionsAtPosition(tsPath, offset, options);

      if (memberAccess && (!result?.entries?.length || result.entries.every((e) => isKeywordEntry(e.name)))) {
        const dotOffset = Math.max(0, offset - 1);
        result = this.service.getCompletionsAtPosition(tsPath, dotOffset, { triggerCharacter: '.' }) || result;
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

  async getCompletionDetails(esPath, line, column, name, source) {
    const tsPath = this.tsPathForEs(esPath);
    const generated = await this.getGeneratedPosition(esPath, line, column);
    return this.service.getCompletionEntryDetails(
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
    return this.service.getQuickInfoAtPosition(
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
    return this.service.getDefinitionAndBoundSpan(
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
    return this.service.getSignatureHelpItems(
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
      ...this.service.getSyntacticDiagnostics(tsPath),
      ...(options.semantic === false ? [] : this.service.getSemanticDiagnostics(tsPath)),
    ];
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
        message: ts.flattenDiagnosticMessageText(diag.messageText, '\n'),
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
