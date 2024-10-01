import {Optimizer} from '@atlaspack/plugin';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {blobToString} from '@atlaspack/utils';

// @ts-expect-error - TS7016 - Could not find a declaration file for module 'svgo'. '/home/ubuntu/parcel/node_modules/svgo/lib/svgo-node.js' implicitly has an 'any' type.
import * as svgo from 'svgo';

export default new Optimizer({
  async loadConfig({config}) {
    let configFile = await config.getConfig([
      'svgo.config.js',
      'svgo.config.cjs',
      'svgo.config.mjs',
      'svgo.config.json',
    ]);

    return configFile?.contents;
  },

  async optimize({bundle, contents, config}) {
    if (!bundle.env.shouldOptimize) {
      return {contents};
    }

    let code = await blobToString(contents);
    let result = svgo.optimize(code, {
      plugins: [
        {
          name: 'preset-default',
          params: {
            overrides: {
              // Removing ids could break SVG sprites.
              cleanupIDs: false,
              // <style> elements and attributes are already minified before they
              // are re-inserted by the packager.
              minifyStyles: false,
            },
          },
        },
      ],
      // @ts-expect-error - TS2698 - Spread types may only be created from object types.
      ...config,
    });

    if (result.error != null) {
      throw new ThrowableDiagnostic({
        diagnostic: {
          message: result.error,
        },
      });
    }

    return {contents: result.data};
  },
}) as Optimizer;
