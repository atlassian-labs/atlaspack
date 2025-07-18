// @flow

import path from 'path';
import {Transformer} from '@atlaspack/plugin';
import {getFeatureFlag} from '@atlaspack/feature-flags';

function shouldExclude(asset, options) {
  return (
    !asset.isSource ||
    !options.hmrOptions ||
    !asset.env.isBrowser() ||
    asset.env.isLibrary ||
    asset.env.isWorker() ||
    asset.env.isWorklet() ||
    options.mode !== 'development' ||
    !asset
      .getDependencies()
      .find(
        (v) =>
          v.specifier === 'react' ||
          v.specifier === 'react/jsx-runtime' ||
          v.specifier === 'react/jsx-dev-runtime' ||
          v.specifier === '@emotion/react' ||
          v.specifier === '@emotion/react/jsx-runtime' ||
          v.specifier === '@emotion/react/jsx-dev-runtime',
      )
  );
}

export default (new Transformer({
  async transform({asset, options}) {
    if (shouldExclude(asset, options)) {
      return [asset];
    }

    const helperFilename = getFeatureFlag('hmrImprovements')
      ? 'helpers-new.js'
      : 'helpers.js';

    let wrapperPath = `@atlaspack/transformer-react-refresh-wrap/${path.basename(
      __dirname,
    )}/helpers/${helperFilename}`;

    let code = await asset.getCode();
    let map = await asset.getMap();
    let name = `$parcel$ReactRefreshHelpers$${asset.id.slice(-4)}`;

    code = `var ${name} = require(${JSON.stringify(wrapperPath)});
${getFeatureFlag('hmrImprovements') ? `${name}.init();` : ''}
var prevRefreshReg = window.$RefreshReg$;
var prevRefreshSig = window.$RefreshSig$;
${name}.prelude(module);

try {
${code}
  ${name}.postlude(module);
} finally {
  window.$RefreshReg$ = prevRefreshReg;
  window.$RefreshSig$ = prevRefreshSig;
}`;

    asset.setCode(code);
    if (map) {
      map.offsetLines(1, 6);
      asset.setMap(map);
    }

    // The JSTransformer has already run, do it manually
    asset.addDependency({
      specifier: wrapperPath,
      specifierType: 'esm',
      resolveFrom: __filename,
    });

    return [asset];
  },
}): Transformer<mixed>);
