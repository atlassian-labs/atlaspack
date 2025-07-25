import type {FilePath, PackageName, Semver} from '@atlaspack/types';
import type {AtlaspackOptions} from './types';

import path from 'path';
import nullthrows from 'nullthrows';
import ThrowableDiagnostic, {
  generateJSONCodeHighlights,
  md,
} from '@atlaspack/diagnostic';
import {findAlternativeNodeModules} from '@atlaspack/utils';
import {ProjectPath, toProjectPath} from './projectPath';

const NODE_MODULES = `${path.sep}node_modules${path.sep}`;
const CONFIG = Symbol.for('parcel-plugin-config');

export default async function loadPlugin<T>(
  pluginName: PackageName,
  configPath: FilePath,
  keyPath: string | null | undefined,
  options: AtlaspackOptions,
): Promise<{
  plugin: T;
  version: Semver;
  resolveFrom: ProjectPath;
}> {
  let resolveFrom = configPath;

  // Config packages can reference plugins, but cannot contain other plugins within them.
  // This forces every published plugin to be published separately so they can be mixed and matched if needed.
  if (resolveFrom.includes(NODE_MODULES) && pluginName.startsWith('.')) {
    let configContents = await options.inputFS.readFile(configPath, 'utf8');
    throw new ThrowableDiagnostic({
      diagnostic: {
        // @ts-expect-error TS2345
        message: md`Local plugins are not supported in Atlaspack config packages. Please publish "${pluginName}" as a separate npm package.`,
        origin: '@atlaspack/core',
        codeFrames: keyPath
          ? [
              {
                filePath: configPath,
                language: 'json5',
                code: configContents,
                codeHighlights: generateJSONCodeHighlights(configContents, [
                  {
                    key: keyPath,
                    type: 'value',
                  },
                ]),
              },
            ]
          : undefined,
      },
    });
  }

  let pkg;
  try {
    ({pkg} = await options.packageManager.resolve(pluginName, resolveFrom, {
      shouldAutoInstall: options.shouldAutoInstall,
    }));
  } catch (err: any) {
    if (err.code !== 'MODULE_NOT_FOUND') {
      throw err;
    }

    let configContents = await options.inputFS.readFile(configPath, 'utf8');
    let alternatives = await findAlternativeNodeModules(
      options.inputFS,
      pluginName,
      path.dirname(resolveFrom),
    );
    throw new ThrowableDiagnostic({
      diagnostic: {
        // @ts-expect-error TS2345
        message: md`Cannot find Atlaspack plugin "${pluginName}"`,
        origin: '@atlaspack/core',
        codeFrames: keyPath
          ? [
              {
                filePath: configPath,
                language: 'json5',
                code: configContents,
                codeHighlights: generateJSONCodeHighlights(configContents, [
                  {
                    key: keyPath,
                    type: 'value',
                    // @ts-expect-error TS2345
                    message: md`Cannot find module "${pluginName}"${
                      alternatives[0]
                        ? `, did you mean "${alternatives[0]}"?`
                        : ''
                    }`,
                  },
                ]),
              },
            ]
          : undefined,
      },
    });
  }

  let plugin = await options.packageManager.require(pluginName, resolveFrom, {
    shouldAutoInstall: options.shouldAutoInstall,
  });
  plugin = plugin.default ? plugin.default : plugin;
  if (!plugin) {
    throw new Error(`Plugin ${pluginName} has no exports.`);
  }
  plugin = plugin[CONFIG];
  if (!plugin) {
    throw new Error(
      `Plugin ${pluginName} is not a valid Atlaspack plugin, should export an instance of a Atlaspack plugin ex. "export default new Reporter({ ... })".`,
    );
  }
  return {
    plugin,
    version: nullthrows(pkg).version,
    resolveFrom: toProjectPath(options.projectRoot, resolveFrom),
  };
}
