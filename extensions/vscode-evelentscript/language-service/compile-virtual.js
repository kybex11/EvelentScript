'use strict';

const fs = require('fs');
const path = require('path');

/**
 * Resolve the EvelentScript compiler from the workspace or this repo.
 */
function resolveEvelentScript(workspaceRoot) {
  const extensionRoot = path.join(__dirname, '..');
  const candidates = [
    path.join(extensionRoot, 'bundled/evelentscript'),
    workspaceRoot && path.join(workspaceRoot, 'lib/evelentscript'),
    workspaceRoot && path.join(workspaceRoot, 'node_modules/evelentscript/lib/evelentscript'),
    workspaceRoot && path.join(workspaceRoot, 'node_modules/evelentscript'),
    path.join(extensionRoot, '..', '..', 'lib/evelentscript'),
  ].filter(Boolean);

  for (const candidate of candidates) {
    try {
      return require(candidate);
    } catch (_) {
      // try next candidate
    }
  }

  throw new Error(
    'EvelentScript compiler not found. Install evelentscript in the workspace or open the EvelentScript repo.'
  );
}

function resolveTypesyntax(workspaceRoot) {
  const extensionRoot = path.join(__dirname, '..');
  const candidates = [
    path.join(extensionRoot, 'bundled/evelentscript/typesyntax'),
    workspaceRoot && path.join(workspaceRoot, 'lib/evelentscript/typesyntax'),
    path.join(extensionRoot, '..', '..', 'lib/evelentscript/typesyntax'),
  ].filter(Boolean);

  for (const candidate of candidates) {
    try {
      return require(candidate);
    } catch (_) {
      // try next candidate
    }
  }

  throw new Error('EvelentScript typesyntax module not found.');
}

/**
 * Pad incomplete syntax so the parser can produce JS/TS for IntelliSense.
 */
function padIncompleteSyntax(source) {
  let lines = source.split('\n');
  lines = lines.map((line) => {
    if (/\.[\w$]*\s*$/.test(line)) {
      return line.replace(/\.([\w$]*)?\s*$/, (_, partial) => (partial ? `.${partial}` : '.exports'));
    }
    return line;
  });
  let result = lines.join('\n');

  const opens = (result.match(/\(/g) || []).length;
  const closes = (result.match(/\)/g) || []).length;
  if (opens > closes) {
    result += ')'.repeat(opens - closes);
  }

  const openBrackets = (result.match(/\[/g) || []).length;
  const closeBrackets = (result.match(/\]/g) || []).length;
  if (openBrackets > closeBrackets) {
    result += ']'.repeat(openBrackets - closeBrackets);
  }

  return result;
}

/**
 * Compile EvelentScript source to a virtual TypeScript file for the language service.
 */
function compileToVirtualTypeScript(source, filename, workspaceRoot) {
  const EvelentScript = resolveEvelentScript(workspaceRoot);
  const { collectTypeDeclarations, emitTypeScriptDeclarations } = resolveTypesyntax(workspaceRoot);

  const helpers = EvelentScript.helpers;
  const literate = helpers?.isLiterate?.(filename) ?? false;
  const paddedSource = padIncompleteSyntax(source);

  let result;
  try {
    result = EvelentScript.compile(paddedSource, {
      filename,
      bare: true,
      header: false,
      sourceMap: true,
      literate,
    });
  } catch (firstError) {
    try {
      result = EvelentScript.compile(`${paddedSource}\n`, {
        filename,
        bare: true,
        header: false,
        sourceMap: true,
        literate,
      });
    } catch (_) {
      throw firstError;
    }
  }

  const js = result.js || result;
  const v3SourceMap =
    typeof result.v3SourceMap === 'string'
      ? JSON.parse(result.v3SourceMap)
      : result.v3SourceMap || null;

  const root = EvelentScript.nodes(paddedSource, {
    filename,
    literate,
  });
  const declarations = collectTypeDeclarations(root.body || root);
  const header = emitTypeScriptDeclarations(declarations);

  // The compiler may emit a leading `//` comment line; strip it and remember
  // how many lines were removed so positions can be mapped accurately.
  const rawJs = String(js);
  const strippedLines = /^\/\/[^\n]*\n/.test(rawJs) ? 1 : 0;
  const body = rawJs.replace(/^\/\/[^\n]*\n/, '');

  const prefixParts = [
    '/// <reference lib="es2020" />',
    '/// <reference types="node" />',
    header || null,
  ].filter((line) => line != null && line !== '');

  // 1-based line in `content` where the compiled body begins.
  const bodyStartLine = prefixParts.length
    ? prefixParts.join('\n').split('\n').length + 1
    : 1;

  const content = prefixParts.length ? `${prefixParts.join('\n')}\n${body}` : body;

  return { content, v3SourceMap, bodyStartLine, strippedLines };
}

function virtualTsPath(esPath) {
  return `${esPath}.evelent.ts`;
}

/**
 * Compile the raw (unpadded) source purely to detect genuine syntax errors.
 * Returns the thrown SyntaxError, or null when the source parses cleanly.
 */
function getSyntaxError(source, filename, workspaceRoot) {
  let EvelentScript;
  try {
    EvelentScript = resolveEvelentScript(workspaceRoot);
  } catch (_) {
    return null;
  }
  const literate = EvelentScript.helpers?.isLiterate?.(filename) ?? false;
  try {
    EvelentScript.compile(source, {
      filename,
      bare: true,
      header: false,
      sourceMap: false,
      literate,
    });
    return null;
  } catch (error) {
    return error;
  }
}

module.exports = {
  compileToVirtualTypeScript,
  resolveEvelentScript,
  getSyntaxError,
  virtualTsPath,
};
