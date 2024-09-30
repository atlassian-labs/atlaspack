// @flow

import * as babel from '@babel/core';
import assert from 'assert';

const plugin = require.resolve('../src/index.js');

describe('@atlaspack/babel-plugin-transform-dev-conditional-imports', () => {
  it('should transform to inline requires', () => {
    const input = `
      importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
    `;
    let {code: transformed} = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [plugin],
    });

    assert(
      transformed ===
        "globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? require('IF_TRUE').default : require('IF_FALSE').default;",
    );
  });
});
