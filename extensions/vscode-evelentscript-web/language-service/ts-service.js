'use strict';

const fs = require('fs');
const path = require('path');
const ts = require('typescript');
const { compileToVirtualTypeScript, virtualTsPath } = require('./compile-virtual');
const { mapToGenerated } = require('./mapping');
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
      lib: ['ES2020'],
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
        return text != null ? ts.ScriptSnapshot.fromString(text) : undefined;
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
    try {
      ({ content, v3SourceMap } = compileToVirtualTypeScript(source, esPath, this.workspaceRoot));
    } catch (_) {
      content = [
        '/// <reference lib="es2020" />',
        '/// <reference types="node" />',
        '',
      ].join('\n');
    }
    const tsPath = this.toPath(virtualTsPath(esPath));
    const existing = this.documents.get(tsPath);
    const version = existing ? existing.version + 1 : 1;
    this.documents.set(tsPath, {
      content,
      version,
      sourceMap: v3SourceMap,
      esPath,
    });
    this.projectVersion += 1;
    return tsPath;
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
    return mapToGenerated(doc.sourceMap, esPath, line, column);
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
