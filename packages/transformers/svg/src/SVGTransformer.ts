import {Transformer} from '@atlaspack/plugin';
import nullthrows from 'nullthrows';
import semver from 'semver';
import {parser as parse} from 'posthtml-parser';
import {render} from 'posthtml-render';
import collectDependencies from './dependencies';
import extractInlineAssets from './inline';
import ThrowableDiagnostic from '@atlaspack/diagnostic';

export default new Transformer({
  canReuseAST({ast}) {
    return ast.type === 'posthtml' && semver.satisfies(ast.version, '^0.4.0');
  },

  async parse({asset}) {
    return {
      type: 'posthtml',
      version: '0.4.1',
      program: parse(await asset.getCode(), {
        directives: [
          {
            name: /^\?/,
            start: '<',
            end: '>',
          },
        ],
        sourceLocations: true,
        xmlMode: true,
      }),
    };
  },

  async transform({asset}) {
    asset.bundleBehavior = 'isolated';

    const ast = nullthrows(await asset.getAST());

    // Check if we're running in v3 mode (where addURLDependency and addDependency are not supported)
    const isV3 = process.env.ATLASPACK_V3 === 'true';

    try {
      // Only collect dependencies if not in v3 mode, since v3 doesn't support addURLDependency yet
      if (!isV3) {
        collectDependencies(asset, ast);
      }
    } catch (errors: any) {
      // Handle both array of errors (from collectDependencies) and single errors (from v3)
      const errorArray = Array.isArray(errors) ? errors : [errors];

      throw new ThrowableDiagnostic({
        diagnostic: errorArray.map((error) => ({
          message: error.message,
          origin: '@atlaspack/transformer-svg',
          codeFrames: [
            {
              filePath: error.filePath,
              language: 'svg',
              codeHighlights: [error.loc],
            },
          ],
        })),
      });
    }

    // Only extract inline assets if not in v3 mode, since v3 doesn't support addDependency yet
    const inlineAssets = isV3 ? [] : extractInlineAssets(asset, ast);

    return [asset, ...inlineAssets];
  },

  generate({ast}) {
    return {
      content: render(ast.program),
    };
  },
}) as Transformer<unknown>;
