// @flow strict-local

import {type PostHTMLNode, render} from 'posthtml-render';
import {parseHTML, transformerOpts} from '../src/HTMLTransformer';
import assert from 'assert';
import type {PluginOptions} from '../../../core/types-internal/src';

function normalizeHTML(code: string): string {
  const ast = parseHTML(code, true);
  // $FlowFixMe
  const result = renderHTML(ast);
  const lines = result
    .split('\n')
    .map(line => line.trim())
    .filter(line => line);
  return lines.join('');
}

function renderHTML(newAST: {|program: PostHTMLNode|}): string {
  return render(newAST.program, {
    closingSingleTag: 'slash',
  });
}

async function runTestTransform(
  code: string,
  options: {|
    shouldScopeHoist: boolean,
    supportsEsmodules: boolean,
    hmrOptions: PluginOptions['hmrOptions'],
  |} = {
    shouldScopeHoist: true,
    supportsEsmodules: true,
    hmrOptions: null,
  },
) {
  const dependencies = [];
  let newAST = null;
  const asset = {
    getAST: () => parseHTML(code, true),
    setAST: n => {
      newAST = n;
    },
    addURLDependency(url, opts) {
      dependencies.push({url, opts});
      return `dependency-id::${url}`;
    },
    env: {
      shouldScopeHoist: options.shouldScopeHoist,
      supports(tag: string, defaultValue: boolean) {
        assert.equal(tag, 'esmodules');
        assert.equal(defaultValue, true);
        return options.supportsEsmodules;
      },
    },
  };

  const transformInput = {
    asset,
    options: {
      hmrOptions: options.hmrOptions,
    },
  };
  // $FlowFixMe
  const transformResult = await transformerOpts.transform(transformInput);

  // $FlowFixMe
  const outputCode = renderHTML(newAST);

  return {dependencies, newAST, outputCode, transformResult, inputAsset: asset};
}

function normalizeDependencies(dependencies) {
  return dependencies.map(dependency => ({
    ...dependency,
    opts: {
      ...dependency.opts,
      env: {
        ...dependency.opts.env,
        loc: null,
      },
    },
  }));
}

describe('HTMLTransformer', () => {
  it('transform simple script tag', async () => {
    const code = `
<html>
  <body>
    <script src="input.js"></script>
  </body>
</html>
    `;
    const {dependencies, outputCode, transformResult, inputAsset} =
      await runTestTransform(code);
    assert.equal(
      outputCode,
      `
<html>
  <body>
    <script src="dependency-id"></script>
  </body>
</html>
    `,
    );
    assert.deepEqual(normalizeDependencies(dependencies), [
      {
        url: 'input.js',
        opts: {
          bundleBehavior: 'isolated',
          env: {
            loc: null,
            outputFormat: 'global',
            sourceType: 'script',
          },
          priority: 'parallel',
        },
      },
    ]);

    assert.deepEqual(transformResult, [inputAsset]);
  });

  it('we will get one dependency per asset', async () => {
    const code = `
<html>
  <body>
    <script src="input1.js"></script>
    <script src="input2.js"></script>
  </body>
</html>
    `;
    const {dependencies, outputCode, transformResult, inputAsset} =
      await runTestTransform(code);
    assert.equal(
      outputCode,
      `
<html>
  <body>
    <script src="dependency-id::input1.js"></script>
    <script src="dependency-id::input2.js"></script>
  </body>
</html>
    `,
    );
    const opts = {
      bundleBehavior: 'isolated',
      env: {
        loc: null,
        outputFormat: 'global',
        sourceType: 'script',
      },
      priority: 'parallel',
    };
    assert.deepEqual(normalizeDependencies(dependencies), [
      {
        url: 'input1.js',
        opts,
      },
      {
        url: 'input2.js',
        opts,
      },
    ]);

    assert.deepEqual(transformResult, [inputAsset]);
  });

  it('transform simple module tag', async () => {
    const code = `
<html>
  <body>
    <script src="input.js" type="module"></script>
  </body>
</html>
    `;
    const {dependencies, outputCode, transformResult, inputAsset} =
      await runTestTransform(code);
    assert.equal(
      outputCode,
      `
<html>
  <body>
    <script src="dependency-id::input.js" type="module"></script>
  </body>
</html>
    `,
    );
    assert.deepEqual(normalizeDependencies(dependencies), [
      {
        url: 'input.js',
        opts: {
          bundleBehavior: undefined,
          env: {
            loc: null,
            outputFormat: 'esmodule',
            sourceType: 'module',
          },
          priority: 'parallel',
        },
      },
    ]);

    assert.deepEqual(transformResult, [inputAsset]);
  });

  it('transform simple module tag if there is no support for esmodules', async () => {
    const code = `
<html>
  <body>
    <script src="input.js" type="module"></script>
  </body>
</html>
    `;
    const {dependencies, outputCode, transformResult, inputAsset} =
      await runTestTransform(code, {
        shouldScopeHoist: true,
        supportsEsmodules: false,
        hmrOptions: null,
      });
    assert.equal(
      normalizeHTML(outputCode),
      normalizeHTML(`
<html>
  <body>
    <script src="dependency-id::input.js" type="module"></script>
    <script src="dependency-id::input.js" nomodule="" defer=""></script>
  </body>
</html>
    `),
    );
    assert.deepEqual(normalizeDependencies(dependencies), [
      {
        url: 'input.js',
        opts: {
          bundleBehavior: undefined,
          env: {
            loc: null,
            outputFormat: 'global',
            sourceType: 'module',
          },
          priority: 'parallel',
        },
      },
      {
        url: 'input.js',
        opts: {
          bundleBehavior: undefined,
          env: {
            loc: null,
            outputFormat: 'esmodule',
            sourceType: 'module',
          },
          priority: 'parallel',
        },
      },
    ]);

    assert.deepEqual(transformResult, [inputAsset]);
  });

  it.only('adds an HMR tag if there are HMR options set', async () => {
    const code = `
<html>
  <body>
    <script src="input.js"></script>
  </body>
</html>
    `;
    const {dependencies, outputCode, transformResult, inputAsset} =
      await runTestTransform(code, {
        shouldScopeHoist: true,
        supportsEsmodules: true,
        hmrOptions: {
          port: 1234,
          host: 'localhost',
        },
      });
    assert.equal(
      normalizeHTML(outputCode),
      normalizeHTML(`
<html>
  <body>
    <script src="dependency-id::input.js"></script>
    <script src="dependency-id::hmr.js"></script>
  </body>
</html>
    `),
    );
    assert.deepEqual(normalizeDependencies(dependencies), [
      {
        url: 'input.js',
        opts: {
          bundleBehavior: 'isolated',
          env: {
            loc: null,
            outputFormat: 'global',
            sourceType: 'script',
          },
          priority: 'parallel',
        },
      },
      {
        url: 'hmr.js',
        opts: {
          env: {
            loc: null,
          },
          priority: 'parallel',
        },
      },
    ]);

    assert.deepEqual(transformResult, [
      inputAsset,
      {
        content: '',
        type: 'js',
        uniqueKey: 'hmr.js',
      },
    ]);
  });
});
