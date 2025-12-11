import {Runtime} from '@atlaspack/plugin';
import {urlJoin} from '@atlaspack/utils';
import type {Asset} from '@atlaspack/types';

export default new Runtime({
  apply({bundle, bundleGraph}) {
    if (bundle.env.context !== 'service-worker') {
      return [];
    }

    let asset = bundle.traverse<Asset>((node, _, actions) => {
      if (
        node.type === 'dependency' &&
        node.value.specifier === '@atlaspack/service-worker' &&
        !bundleGraph.isDependencySkipped(node.value)
      ) {
        actions.stop();
        return bundleGraph.getResolvedAsset(node.value, bundle);
      }
    });

    if (!asset) {
      return [];
    }

    let manifest: Array<string> = [];
    bundleGraph.traverseBundles((b) => {
      if (b.bundleBehavior === 'inline' || b.id === bundle.id) {
        return;
      }

      manifest.push(urlJoin(b.target.publicUrl, b.name));
    });

    let code = `import {_register} from '@atlaspack/service-worker';
const manifest = ${JSON.stringify(manifest)};
const version = ${JSON.stringify(bundle.hashReference)};
_register(manifest, version);
`;

    return [
      {
        filePath: asset.filePath,
        code,
        isEntry: true,
        env: {sourceType: 'module'},
        // Pre-computed symbols: imports _register from @atlaspack/service-worker, no exports
        symbolData: {
          symbols: new Map(), // No exports, just side effects
          dependencies: [
            {
              specifier: '@atlaspack/service-worker',
              symbols: new Map([
                ['_register', {local: '_register', loc: null, isWeak: false}],
              ]),
              usedSymbols: new Set(['_register']),
            },
          ],
        },
      },
    ];
  },
}) as Runtime<unknown>;
