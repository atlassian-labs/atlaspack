import type {Config} from '@atlaspack/types';
import path from 'path';

type ConfigResult = {
  config: any;
};

export async function load({config}: {config: Config}): Promise<ConfigResult> {
  let configFile = await config.getConfig(
    ['.lessrc', '.lessrc.js', '.lessrc.cjs', '.lessrc.mjs'],
    {
      packageKey: 'less',
    },
  );

  let configContents: Record<string, any> = {};
  if (configFile != null) {
    // @ts-expect-error - TS2322 - Type 'unknown' is not assignable to type 'Record<string, any>'.
    configContents = configFile.contents;

    // Resolve relative paths from config file
    if (configContents.paths) {
      // @ts-expect-error - TS7006 - Parameter 'p' implicitly has an 'any' type.
      configContents.paths = configContents.paths.map((p) =>
        // @ts-expect-error - TS2533 - Object is possibly 'null' or 'undefined'.
        path.resolve(path.dirname(configFile.filePath), p),
      );
    }
  }

  // Rewrites urls to be relative to the provided filename
  configContents.rewriteUrls = 'all';
  configContents.plugins = configContents.plugins || [];

  return {config: configContents};
}
