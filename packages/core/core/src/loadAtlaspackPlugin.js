// @flow
import type {
  FilePath,
  PackageName,
  Semver,
  SemverRange,
} from '@atlaspack/types';
import type {AtlaspackOptions} from './types';

import path from 'path';
import semver from 'semver';
import logger from '@atlaspack/logger';
import nullthrows from 'nullthrows';
import ThrowableDiagnostic, {
  generateJSONCodeHighlights,
  md,
} from '@atlaspack/diagnostic';
import {
  findAlternativeNodeModules,
  loadConfig,
  resolveConfig,
} from '@atlaspack/utils';
import {type ProjectPath, toProjectPath} from './projectPath';
import {version as ATLASPACK_VERSION} from '../package.json';

const NODE_MODULES = `${path.sep}node_modules${path.sep}`;
const CONFIG = Symbol.for('atlaspack-plugin-config');

export default async function loadPlugin<T>(
  pluginName: PackageName,
  configPath: FilePath,
  keyPath?: string,
  options: AtlaspackOptions,
): Promise<{|
  plugin: T,
  version: Semver,
  resolveFrom: ProjectPath,
  range: ?SemverRange,
|}> {
  let resolveFrom = configPath;
  let range;
  if (resolveFrom.includes(NODE_MODULES)) {
    // Config packages can reference plugins, but cannot contain other plugins within them.
    // This forces every published plugin to be published separately so they can be mixed and matched if needed.
    if (pluginName.startsWith('.')) {
      let configContents = await options.inputFS.readFile(configPath, 'utf8');
      throw new ThrowableDiagnostic({
        diagnostic: {
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

    let configPkg = await loadConfig(
      options.inputFS,
      resolveFrom,
      ['package.json'],
      options.projectRoot,
    );
    if (
      configPkg != null &&
      configPkg.config.dependencies?.[pluginName] == null
    ) {
      // If not in the config's dependencies, the plugin will be auto installed with
      // the version declared in "atlaspackDependencies".
      range = configPkg.config.atlaspackDependencies?.[pluginName];

      if (range == null) {
        let contents = await options.inputFS.readFile(
          configPkg.files[0].filePath,
          'utf8',
        );
        throw new ThrowableDiagnostic({
          diagnostic: {
            message: md`Could not determine version of ${pluginName} in ${path.relative(
              process.cwd(),
              resolveFrom,
            )}. Either include it in "dependencies" or "atlaspackDependencies".`,
            origin: '@atlaspack/core',
            codeFrames:
              configPkg.config.dependencies ||
              configPkg.config.atlaspackDependencies
                ? [
                    {
                      filePath: configPkg.files[0].filePath,
                      language: 'json5',
                      code: contents,
                      codeHighlights: generateJSONCodeHighlights(contents, [
                        {
                          key: configPkg.config.atlaspackDependencies
                            ? '/atlaspackDependencies'
                            : '/dependencies',
                          type: 'key',
                        },
                      ]),
                    },
                  ]
                : undefined,
          },
        });
      }

      // Resolve from project root if not in the config's dependencies.
      resolveFrom = path.join(options.projectRoot, 'index');
    }
  }

  let resolved, pkg;
  try {
    ({resolved, pkg} = await options.packageManager.resolve(
      pluginName,
      resolveFrom,
      {
        shouldAutoInstall: options.shouldAutoInstall,
        range,
      },
    ));
  } catch (err) {
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

  // Remove plugin version compatiblility validation in canary builds as they don't use semver
  if (!process.env.SKIP_PLUGIN_COMPATIBILITY_CHECK) {
    if (!pluginName.startsWith('.')) {
      // Validate the engines.atlaspack field in the plugin's package.json
      let atlaspackVersionRange = pkg && pkg.engines && pkg.engines.atlaspack;
      if (!atlaspackVersionRange) {
        logger.warn({
          origin: '@atlaspack/core',
          message: `The plugin "${pluginName}" needs to specify a \`package.json#engines.atlaspack\` field with the supported Atlaspack version range.`,
        });
      }

      if (
        atlaspackVersionRange &&
        !semver.satisfies(ATLASPACK_VERSION, atlaspackVersionRange)
      ) {
        let pkgFile = nullthrows(
          await resolveConfig(
            options.inputFS,
            resolved,
            ['package.json'],
            options.projectRoot,
          ),
        );
        let pkgContents = await options.inputFS.readFile(pkgFile, 'utf8');
        throw new ThrowableDiagnostic({
          diagnostic: {
            message: md`The plugin "${pluginName}" is not compatible with the current version of Atlaspack. Requires "${atlaspackVersionRange}" but the current version is "${ATLASPACK_VERSION}".`,
            origin: '@atlaspack/core',
            codeFrames: [
              {
                filePath: pkgFile,
                language: 'json5',
                code: pkgContents,
                codeHighlights: generateJSONCodeHighlights(pkgContents, [
                  {
                    key: '/engines/atlaspack',
                  },
                ]),
              },
            ],
          },
        });
      }
    }
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
    range,
  };
}