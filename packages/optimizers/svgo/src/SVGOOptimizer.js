// @flow

import {Optimizer} from '@atlaspack/plugin';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {blobToString} from '@atlaspack/utils';

import * as svgo from 'svgo';

export default (new Optimizer({
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
}): Optimizer<mixed, mixed>);
