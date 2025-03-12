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
