// @flow

import * as babel from '@babel/core';
import assert from 'assert';

const plugin = require.resolve('../src/index.ts');

describe('@atlaspack/babel-plugin-transform-contextual-imports', () => {
  it('should transform importCond to inline requires', () => {
    const input = `
      const Imported = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
    `;
    const {code: transformed} = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [plugin],
    });

    assert.equal(
      transformed,
      "const Imported = globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? require('IF_TRUE').default : require('IF_FALSE').default;",
    );
  });

  it('should transform importCond to ssr safe code', () => {
    const input = `
      const Imported = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');

      console.log(Imported);
    `;
    const {code: transformed} = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {node: true}]],
    });

    assert.equal(
      transformed,
      `const _CONDITION$IF_TRUE$IF_FALSE = {
  ifTrue: require('IF_TRUE').default,
  ifFalse: require('IF_FALSE').default
};
Object.defineProperty(_CONDITION$IF_TRUE$IF_FALSE, "load", {
  get: () => globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? _CONDITION$IF_TRUE$IF_FALSE.ifTrue : _CONDITION$IF_TRUE$IF_FALSE.ifFalse
});
console.log(_CONDITION$IF_TRUE$IF_FALSE.load);`,
    );
  });
});
