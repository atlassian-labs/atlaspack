import {Optimizer} from '@atlaspack/plugin';
import {blobToString, stripAnsi} from '@atlaspack/utils';
import ThrowableDiagnostic, {escapeMarkdown} from '@atlaspack/diagnostic';
import path from 'path';

import {minifyWithSourceMap} from './minifyWithSourceMap';

export default new Optimizer({
  async loadConfig({config, options}) {
    let userConfig = await config.getConfigFrom(
      path.join(options.projectRoot, 'index'),
      ['.terserrc', '.terserrc.js', '.terserrc.cjs', '.terserrc.mjs'],
    );

    return userConfig?.contents;
  },
  async optimize({
    contents,
    map: originalMap,
    bundle,
    config: userConfig,
    options,
    getSourceMapReference,
  }) {
    if (!bundle.env.shouldOptimize) {
      return {contents, map: originalMap};
    }

    let code = await blobToString(contents);
    let result;
    try {
      result = await minifyWithSourceMap(
        code,
        bundle.env.sourceMap ? (originalMap ?? null) : null,
        {
          target: 'es2022',
          mangle: true,
          compress: true,
          toplevel:
            bundle.env.outputFormat === 'esmodule' ||
            bundle.env.outputFormat === 'commonjs',
          module: bundle.env.outputFormat === 'esmodule',
          userConfig: userConfig as Record<string, unknown> | undefined,
          projectRoot: options.projectRoot,
        },
      );
    } catch (err: any) {
      // SWC doesn't give us nice error objects, so we need to parse the message.
      let message = escapeMarkdown(
        (
          stripAnsi(err.message)
            .split('\n')
            .find((line) => line.trim().length > 0) || ''
        )
          .trim()
          .replace(/^(×|x)\s+/, ''),
      );
      let location = err.message.match(/(?:╭─|,-)\[(\d+):(\d+)\]/);
      if (location) {
        let line = Number(location[1]);
        let col = Number(location[1]);
        let mapping = originalMap?.findClosestMapping(line, col);
        if (mapping && mapping.original && mapping.source) {
          let {source, original} = mapping;
          let filePath = path.resolve(options.projectRoot, source);
          throw new ThrowableDiagnostic({
            diagnostic: {
              message,
              origin: '@atlaspack/optimizer-swc',
              codeFrames: [
                {
                  language: 'js',
                  filePath,
                  codeHighlights: [{start: original, end: original}],
                },
              ],
            },
          });
        }

        let loc = {
          line: line,
          column: col,
        };

        throw new ThrowableDiagnostic({
          diagnostic: {
            message,
            origin: '@atlaspack/optimizer-swc',
            codeFrames: [
              {
                language: 'js',
                filePath: undefined,
                code,
                codeHighlights: [{start: loc, end: loc}],
              },
            ],
          },
        });
      }

      throw err;
    }

    let minifiedContents: string = result.code;
    const sourceMap = result.map;
    if (sourceMap) {
      let sourcemapReference = await getSourceMapReference(sourceMap);
      if (sourcemapReference) {
        minifiedContents += `\n//# sourceMappingURL=${sourcemapReference}\n`;
      }
    }

    return {contents: minifiedContents, map: sourceMap};
  },
}) as Optimizer<unknown, unknown>;
