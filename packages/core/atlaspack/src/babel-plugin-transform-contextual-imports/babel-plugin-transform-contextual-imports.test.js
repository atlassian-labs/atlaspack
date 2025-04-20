// @flow

import * as babel from '@babel/core';
import assert from 'assert';

const plugin = require.resolve('./index.ts');

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

  it('should transform importCond to server (deprecated) lazy code', () => {
    const input = `
      importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
    `;
    let {code: transformed} = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {server: true}]],
    });

    assert(
      transformed ===
        `const _CONDITION$IF_TRUE$IF_FALSE = {
  ifTrue: require('IF_TRUE').default,
  ifFalse: require('IF_FALSE').default
};
Object.defineProperty(_CONDITION$IF_TRUE$IF_FALSE, "load", {
  get: () => globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? _CONDITION$IF_TRUE$IF_FALSE.ifTrue : _CONDITION$IF_TRUE$IF_FALSE.ifFalse
});
_CONDITION$IF_TRUE$IF_FALSE.load;`,
    );
  });

  it('should transform importCond to node lazy code', () => {
    const input = `
      const Imported = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');

      console.log(Imported, Imported.someProperty);
    `;
    const {code: transformed} = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {node: true}]],
    });

    assert.equal(
      transformed,
      `const Imported = {
  ifTrue: require('IF_TRUE').default,
  ifFalse: require('IF_FALSE').default
};
Object.defineProperty(Imported, "load", {
  get: () => globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? Imported.ifTrue : Imported.ifFalse
});
console.log(Imported.load, Imported.load.someProperty);`,
    );
  });
});
