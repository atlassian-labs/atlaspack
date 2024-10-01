import SourceMap from '@parcel/source-map';
import {Optimizer} from '@atlaspack/plugin';
import postcss from 'postcss';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'cssnano'. '/home/ubuntu/parcel/node_modules/cssnano/dist/index.js' implicitly has an 'any' type.
import cssnano from 'cssnano';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'cssnano'. '/home/ubuntu/parcel/node_modules/cssnano/dist/index.js' implicitly has an 'any' type.
import type {CSSNanoOptions} from 'cssnano'; // TODO the type is based on cssnano 4

export default new Optimizer({
  async loadConfig({config}) {
    const configFile = await config.getConfig(
      [
        '.cssnanorc',
        'cssnano.config.json',
        'cssnano.config.js',
        'cssnano.config.cjs',
        'cssnano.config.mjs',
      ],
      {
        packageKey: 'cssnano',
      },
    );
    if (configFile) {
      return configFile.contents;
    }
  },

  async optimize({
    bundle,
    contents: prevContents,
    getSourceMapReference,
    map: prevMap,
    config,
    options,
  }) {
    if (!bundle.env.shouldOptimize) {
      return {contents: prevContents, map: prevMap};
    }

    if (typeof prevContents !== 'string') {
      throw new Error(
        'CSSNanoOptimizer: Only string contents are currently supported',
      );
    }

    const result = await postcss([
      cssnano(config ?? ({} as CSSNanoOptions)),
    ]).process(prevContents, {
      // Suppress postcss's warning about a missing `from` property. In this
      // case, the input map contains all of the sources.
      from: undefined,
      map: {
        annotation: false,
        inline: false,
        // @ts-expect-error - TS2322 - Type 'string | Readonly<{ sources: readonly string[]; sourcesContent?: readonly (string | null)[] | undefined; names: readonly string[]; mappings: string; version?: number | undefined; file?: string | undefined; sourceRoot?: string | undefined; }> | null' is not assignable to type 'string | boolean | object | ((file: string) => string) | undefined'.
        prev: prevMap ? await prevMap.stringify({}) : null,
      },
    });

    let map;
    if (result.map != null) {
      map = new SourceMap(options.projectRoot);
      // @ts-expect-error - TS2345 - Argument of type 'RawSourceMap' is not assignable to parameter of type 'Readonly<{ sources: readonly string[]; sourcesContent?: readonly (string | null)[] | undefined; names: readonly string[]; mappings: string; version?: number | undefined; file?: string | undefined; sourceRoot?: string | undefined; }>'.
      map.addVLQMap(result.map.toJSON());
    }

    let contents = result.css;
    if (bundle.env.sourceMap) {
      let reference = await getSourceMapReference(map);
      if (reference != null) {
        contents += '\n' + '/*# sourceMappingURL=' + reference + ' */\n';
      }
    }

    return {
      contents,
      map,
    };
  },
}) as Optimizer;
