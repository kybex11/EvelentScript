'use strict';

/** @type {Record<string, string[]>} */
const GLOBAL_MEMBERS = {
  module: ['exports', 'require', 'filename', 'id', 'path', 'paths', 'parent', 'children', 'loaded', 'main'],
  process: ['env', 'argv', 'cwd', 'exit', 'platform', 'version', 'pid', 'stdin', 'stdout', 'stderr', 'nextTick'],
  console: ['log', 'error', 'warn', 'info', 'debug', 'trace', 'dir', 'time', 'timeEnd', 'clear', 'assert'],
  Object: ['keys', 'values', 'entries', 'assign', 'create', 'defineProperty', 'freeze', 'fromEntries', 'hasOwn'],
  Array: ['from', 'isArray', 'of'],
  String: ['fromCharCode', 'fromCodePoint', 'raw'],
  Number: ['isFinite', 'isInteger', 'isNaN', 'parseFloat', 'parseInt', 'MAX_VALUE', 'MIN_VALUE'],
  Math: ['abs', 'ceil', 'floor', 'max', 'min', 'random', 'round', 'sqrt', 'pow', 'sign', 'trunc'],
  JSON: ['parse', 'stringify'],
  Date: ['now', 'parse', 'UTC'],
  Promise: ['all', 'allSettled', 'any', 'race', 'reject', 'resolve'],
  global: ['process', 'console', 'Buffer', 'setTimeout', 'setInterval', 'clearTimeout', 'clearInterval'],
  Buffer: ['from', 'alloc', 'concat', 'isBuffer', 'byteLength'],
};

const NODE_BUILTINS = [
  'assert', 'buffer', 'child_process', 'crypto', 'events', 'fs', 'http', 'https',
  'net', 'os', 'path', 'querystring', 'readline', 'stream', 'string_decoder',
  'timers', 'url', 'util', 'vm', 'zlib',
];

const TS_KEYWORDS = new Set([
  'abstract', 'any', 'as', 'asserts', 'async', 'await', 'bigint', 'boolean', 'break',
  'case', 'catch', 'class', 'const', 'continue', 'debugger', 'declare', 'default',
  'delete', 'do', 'else', 'enum', 'export', 'extends', 'false', 'finally', 'for',
  'from', 'function', 'get', 'if', 'implements', 'import', 'in', 'infer', 'instanceof',
  'interface', 'is', 'keyof', 'let', 'module', 'namespace', 'never', 'new', 'null',
  'number', 'object', 'of', 'package', 'private', 'protected', 'public', 'readonly',
  'require', 'return', 'satisfies', 'set', 'static', 'string', 'super', 'switch',
  'symbol', 'this', 'throw', 'true', 'try', 'type', 'typeof', 'undefined', 'unique',
  'unknown', 'using', 'var', 'void', 'while', 'with', 'yield',
]);

function isMemberAccessContext(lineText, character) {
  const prefix = lineText.slice(0, character);
  return /(?:^|[^\w$])([\w$]+)\.\w*$/.test(prefix);
}

function getMemberTarget(lineText, character) {
  const prefix = lineText.slice(0, character);
  const match = prefix.match(/(?:^|[^\w$])([\w$]+)\.\w*$/);
  return match ? match[1] : null;
}

function getFallbackCompletions(lineText, character) {
  const target = getMemberTarget(lineText, character);
  if (target) {
    const members = GLOBAL_MEMBERS[target];
    if (members?.length) {
      return members.map((name) => ({ name, kind: 'property' }));
    }
  }

  const prefix = lineText.slice(0, character);
  const requireMatch = prefix.match(/require\s*\(\s*['"]([\w./-]*)$/);
  if (requireMatch) {
    const partial = requireMatch[1];
    return NODE_BUILTINS
      .filter((name) => name.startsWith(partial))
      .map((name) => ({ name, kind: 'module' }));
  }

  return [];
}

function isKeywordEntry(name) {
  return TS_KEYWORDS.has(name);
}

module.exports = {
  getFallbackCompletions,
  isMemberAccessContext,
  isKeywordEntry,
  GLOBAL_MEMBERS,
};
