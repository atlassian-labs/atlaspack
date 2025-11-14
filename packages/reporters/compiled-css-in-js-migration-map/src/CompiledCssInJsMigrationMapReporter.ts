import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Reporter} from '@atlaspack/plugin';
import {join, relative} from 'node:path';

export default new Reporter({
  async report({event, options}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return;
    }
    if (event.type === 'buildSuccess') {
      const safeAssets: Record<string, string> = {};
      const unsafeAssets: Record<
        string,
        {asset: string; babel: string[]; swc: string[]}
      > = {};
      let total = 0;
      let safe = 0;
      event.bundleGraph.traverseBundles((childBundle) => {
        childBundle.traverseAssets((asset) => {
          if (asset.meta.compiledCodeHash) {
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

            if (mismatches.length === 0) {
              safe += 1;
              safeAssets[asset.meta.compiledCodeHash as string] = relative(
                options.projectRoot,
                asset.filePath,
              );
            } else {
              unsafeAssets[asset.meta.compiledCodeHash as string] = {
                asset: relative(options.projectRoot, asset.filePath),
                babel: Array.from(babelStyleRules),
                swc: Array.from(swcStyleRules),
              };
            }
            total += 1;
          }
        });
      });

      if (total > 0) {
        await options.outputFS.writeFile(
          join(options.projectRoot, 'compiled-css-migration-map.json'),
          JSON.stringify(
            {safeAssets, unsafeAssets, stats: {total, safe}},
            null,
            2,
          ),
        );
      }
    }
  },
}) as Reporter;
