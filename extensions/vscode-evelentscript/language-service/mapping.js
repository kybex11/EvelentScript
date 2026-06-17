'use strict';

const path = require('path');
const { SourceMapConsumer } = require('source-map');

/**
 * Map a position in the original .es file to the generated virtual .ts file.
 */
async function mapToGenerated(sourceMapJson, esPath, esLine, esColumn) {
  if (!sourceMapJson) {
    return { line: esLine, column: esColumn };
  }

  const consumer = await new SourceMapConsumer(sourceMapJson);
  try {
    const source =
      sourceMapJson.sources?.find((name) => name === esPath || name.endsWith(path.basename(esPath))) ||
      sourceMapJson.sources?.[0];

    const generated = consumer.generatedPositionFor({
      source,
      line: esLine,
      column: esColumn,
      bias: SourceMapConsumer.GREATEST_LOWER_BOUND,
    });

    if (generated.line == null) {
      return { line: esLine, column: esColumn };
    }

    return {
      line: generated.line,
      column: generated.column == null ? 0 : generated.column,
    };
  } finally {
    consumer.destroy();
  }
}

/**
 * Map an array of positions in the generated JS back to the original .es file.
 * Uses a single SourceMapConsumer for the whole batch for performance.
 *
 * @param {object|null} sourceMapJson
 * @param {Array<{ line: number, column: number }>} positions generated JS positions (1-based line)
 * @returns {Promise<Array<{ line: number, column: number } | null>>} original .es positions (1-based line) or null when unmapped
 */
async function mapManyToOriginal(sourceMapJson, positions) {
  if (!sourceMapJson) {
    return positions.map(() => null);
  }

  const consumer = await new SourceMapConsumer(sourceMapJson);
  try {
    return positions.map(({ line, column }) => {
      const original = consumer.originalPositionFor({
        line,
        column,
        bias: SourceMapConsumer.GREATEST_LOWER_BOUND,
      });
      if (original.line == null) {
        return null;
      }
      return {
        line: original.line,
        column: original.column == null ? 0 : original.column,
      };
    });
  } finally {
    consumer.destroy();
  }
}

module.exports = {
  mapToGenerated,
  mapManyToOriginal,
};
