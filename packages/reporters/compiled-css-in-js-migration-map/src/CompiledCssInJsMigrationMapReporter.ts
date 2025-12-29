import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Reporter} from '@atlaspack/plugin';
import {join, relative} from 'node:path';

export default new Reporter({
  async report({event, options, logger}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return;
    }
    if (event.type === 'buildSuccess') {
      const currentMap = (await options.outputFS.exists(
        join(options.projectRoot, 'compiled-css-migration-map.json'),
      ))
        ? JSON.parse(
            await options.outputFS.readFile(
              join(options.projectRoot, 'compiled-css-migration-map.json'),
              'utf-8',
            ),
          )
        : {};

      const safeAssets: Record<string, {asset: string}> =
        currentMap?.safeAssets ?? {};
      const unsafeAssets: Record<
        string,
        {
          asset: string;
          babel: string[];
          swc: string[];
          diagnostics: string[];
        }
      > = currentMap?.unsafeAssets ?? {};

      event.bundleGraph.traverseBundles((childBundle) => {
        childBundle.traverseAssets((asset) => {
          if (asset.meta.styleRules) {
            const assetPath = relative(options.projectRoot, asset.filePath);

            const currentSafeAsset = Object.entries(safeAssets).find(
              ([, data]) => data.asset === assetPath,
            );

            if (currentSafeAsset) {
              delete safeAssets[currentSafeAsset[0]];
            }

            const currentUnsafeAsset = Object.entries(unsafeAssets).find(
              ([, data]) => data.asset === assetPath,
            );

            if (currentUnsafeAsset) {
              delete unsafeAssets[currentUnsafeAsset[0]];
            }

            const babelStyleRules = new Set(
              (asset.meta.styleRules as string[]) ?? [],
            );
            const swcStyleRules = new Set(
              (asset.meta.swcStyleRules as string[]) ?? [],
            );

            const mismatches = [];
            for (const rule of [...babelStyleRules, ...swcStyleRules]) {
              if (!babelStyleRules.has(rule) || !swcStyleRules.has(rule)) {
                mismatches.push(rule);
              }
            }

            if (mismatches.length === 0 && !asset.meta.compiledBailOut) {
              if (asset.meta.compiledCodeHash) {
                safeAssets[asset.meta.compiledCodeHash as string] = {
                  asset: relative(options.projectRoot, asset.filePath),
                };
              }
            } else {
              unsafeAssets[
                (asset.meta.compiledCodeHash as string) ??
                  (relative(options.projectRoot, asset.filePath) as string)
              ] = {
                asset: relative(options.projectRoot, asset.filePath),
                babel: Array.from(babelStyleRules).sort(),
                swc: Array.from(swcStyleRules).sort(),
                diagnostics:
                  (asset.meta.compiledCssDiagnostics as string[]) ?? [],
              };
            }
          }
        });
      });

      if (
        Object.keys(safeAssets).length > 0 ||
        Object.keys(unsafeAssets).length > 0
      ) {
        await options.outputFS.writeFile(
          join(options.projectRoot, 'compiled-css-migration-map.json'),
          JSON.stringify(
            {
              safeAssets,
              unsafeAssets,
              stats: {
                total:
                  Object.keys(safeAssets).length +
                  Object.keys(unsafeAssets).length,
                safe: Object.keys(safeAssets).length,
              },
            },
            null,
            2,
          ),
        );
        logger.info({
          message: `Wrote compiled-css-migration-map.json to ${join(options.projectRoot, 'compiled-css-migration-map.json')}`,
        });
      } else {
        logger.info({
          message: 'No compiled CSS in JS assets found',
        });
      }
    }
  },
}) as Reporter;
