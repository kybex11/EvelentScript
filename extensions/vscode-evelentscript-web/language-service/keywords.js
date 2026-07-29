'use strict';

/** EvelentScript keywords */
const KEYWORDS = [
  'and', 'or', 'is', 'isnt', 'not', 'on', 'yes', 'no', 'true', 'false', 'null', 'undefined',
  'if', 'else', 'unless', 'switch', 'when', 'then', 'try', 'catch', 'finally', 'throw',
  'for', 'own', 'of', 'by', 'in', 'while', 'until', 'loop', 'do', 'break', 'continue', 'return',
  'class', 'extends', 'new', 'delete', 'typeof', 'instanceof', 'super', 'this',
  'function', 'async', 'await', 'yield', 'export', 'import', 'from', 'default',
  'interface', 'type', 'readonly', 'void', 'never',
  'debugger', 'break', 'continue',
];

function getWordPrefix(lineText, column) {
  const before = lineText.slice(0, column);
  const match = before.match(/[\w$]+$/);
  return match ? match[0] : '';
}

function getKeywordCompletions(lineText, column) {
  const prefix = getWordPrefix(lineText, column);
  if (!prefix) {
    return KEYWORDS.map((name) => ({ name, kind: 'keyword' }));
  }
  const lower = prefix.toLowerCase();
  return KEYWORDS
    .filter((name) => name.startsWith(lower) && name !== prefix)
    .map((name) => ({ name, kind: 'keyword' }));
}

module.exports = {
  KEYWORDS,
  getKeywordCompletions,
  getWordPrefix,
};
