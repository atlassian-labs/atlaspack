/*
 * These tools are intended for Atlaspack developers, to provide extra utilities
 * to make debugging Atlaspack issues more straightforward.
 *
 * To enable a tool, set the `ATLASPACK_DEBUG_TOOLS` environment variable to a
 * comma-separated list of tool names. For example:
 * `ATLASPACK_DEBUG_TOOLS="asset-file-names-in-output,simple-cli-reporter"`
 *
 * You can enable all tools by setting `ATLASPACK_DEBUG_TOOLS=all`.
 */

type DebugTools = {
  ['asset-file-names-in-output']: boolean;
  ['simple-cli-reporter']: boolean;
};

export let debugTools: DebugTools = {
  'asset-file-names-in-output': false,
  'simple-cli-reporter': false,
};

const envVarValue = process.env.ATLASPACK_DEBUG_TOOLS ?? '';

for (let tool of envVarValue.split(',')) {
  tool = tool.trim();

  if (tool === 'all') {
    for (let key in debugTools) {
      debugTools[key] = true;
    }
    break;
  } else if (debugTools.hasOwnProperty(tool)) {
    debugTools[tool] = true;
  } else if (tool === '') {
    continue;
  } else {
    throw new Error(
      `Invalid debug tool option: ${tool}. Valid options are: ${Object.keys(
        debugTools,
      ).join(', ')}`,
    );
  }
}
