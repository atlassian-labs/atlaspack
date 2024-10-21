import * as babel from '@babel/core';
import assert from 'assert';

const plugin = require.resolve('../src/index.ts');

describe('@atlaspack/babel-plugin-transform-contextual-imports', () => {
  it('should transform importCond to inline requires', () => {
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

  it('should transform importCond to ssr safe code', () => {
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
});
