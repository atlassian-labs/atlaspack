import * as babel from '@babel/core';
import assert from 'assert';

const plugin = require.resolve('../src/index.ts');

describe('@atlaspack/babel-plugin-transform-contextual-imports', () => {
  it('should transform importCond to inline requires', () => {
    const input = `
      const Imported = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
    `;
    const result = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [plugin],
    });

    assert.equal(
      result?.code,
      "const Imported = globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? require('IF_TRUE').default : require('IF_FALSE').default;",
    );
  });

  it('should transform importCond to node lazy code', () => {
    const input = `
      const Imported = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');

      console.log(Imported, Imported.someProperty);
    `;
    const result = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {node: true}]],
    });

    assert.equal(
      result?.code,
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

  it('should transform shadowed identifiers correctly', () => {
    const input = `const JqlUtils = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');

const myObject = {
  JqlUtils: 'some-value',
  other: JqlUtils,
};

const jsx = <MyComponent JqlUtils={JqlUtils} />;`;
    const result = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {node: true, useBindingAwareReplacement: true}]],
      parserOpts: {
        plugins: ['jsx'],
      },
    });

    assert.equal(
      result?.code,
      `const JqlUtils = {
  ifTrue: require('IF_TRUE').default,
  ifFalse: require('IF_FALSE').default
};
Object.defineProperty(JqlUtils, "load", {
  get: () => globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? JqlUtils.ifTrue : JqlUtils.ifFalse
});
const myObject = {
  JqlUtils: 'some-value',
  other: JqlUtils.load
};
const jsx = <MyComponent JqlUtils={JqlUtils.load} />;`,
    );
  });

  it('should use legacy behavior when feature flag is disabled', () => {
    const input = `const JqlUtils = importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
const SomeValue = JqlUtils.someProperty;`;
    const result = babel.transformSync(input, {
      configFile: false,
      presets: [],
      plugins: [[plugin, {node: true, useBindingAwareReplacement: false}]],
    });

    assert.equal(
      result?.code,
      `const JqlUtils = {
  ifTrue: require('IF_TRUE').default,
  ifFalse: require('IF_FALSE').default
};
Object.defineProperty(JqlUtils, "load", {
  get: () => globalThis.__MCOND && globalThis.__MCOND('CONDITION') ? JqlUtils.ifTrue : JqlUtils.ifFalse
});
const SomeValue = JqlUtils.load.someProperty;`,
    );
  });
});
