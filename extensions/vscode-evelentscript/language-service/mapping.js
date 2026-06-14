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

module.exports = {
  mapToGenerated,
};
