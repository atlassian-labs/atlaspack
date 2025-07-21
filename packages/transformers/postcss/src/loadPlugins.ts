import type {FilePath, PluginOptions} from '@atlaspack/types';
import type {PackageManager} from '@atlaspack/package-manager';

export default async function loadExternalPlugins(
  plugins:
    | Array<string>
    | {
        readonly [pluginName: string]: unknown;
      },
  relative: FilePath,
  options: PluginOptions,
): Promise<Array<unknown>> {
  if (Array.isArray(plugins)) {
    return Promise.all(
      plugins
        .map((p) =>
          loadPlugin(
            p,
            relative,
            null,
            options.packageManager,
            options.shouldAutoInstall,
          ),
        )
        .filter(Boolean),
    );
  } else if (typeof plugins === 'object') {
    let _plugins = plugins;
    let mapPlugins = await Promise.all(
      Object.keys(plugins).map((p) =>
        loadPlugin(
          p,
          relative,
          _plugins[p],
          options.packageManager,
          options.shouldAutoInstall,
        ),
      ),
    );
    return mapPlugins.filter(Boolean);
  } else {
    return [];
  }
}

async function loadPlugin(
  pluginArg: string | any,
  relative: FilePath,
  options: unknown | null | undefined = {},
  packageManager: PackageManager,
  shouldAutoInstall: boolean,
): unknown {
  if (typeof pluginArg !== 'string') {
    return pluginArg;
  }

  let plugin = await packageManager.require(pluginArg, relative, {
    shouldAutoInstall,
  });
  plugin = plugin.default || plugin;

  if (
    options != null &&
    typeof options === 'object' &&
    Object.keys(options).length > 0
  ) {
    plugin = plugin(options);
  }

  return plugin.default || plugin;
}
