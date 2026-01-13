import assert from 'assert';
import path from 'path';
import {rimraf} from 'rimraf';
import {
  bundle,
  describe,
  it,
  run,
  overlayFS,
  fsFixture,
  inputFS,
} from '@atlaspack/test-utils';

describe('plugins with "registered" languages', () => {
  it('should support plugins with esbuild-register', async () => {
    const dir = path.join(__dirname, 'esbuild-register-plugin');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      package.json:
        {
          "name": "app",
          "sideEffects": true
        }

      yarn.lock:

      index.js:
        console.log("Hi, mum!");

      .parcelrc:
        {
          extends: "@atlaspack/config-default",
          reporters: ["...", "./reporter-plugin.js"],
        }

      reporter-plugin.js:
        require('esbuild-register/dist/node').register();
        const plugin = require('./reporter-plugin.ts');
        module.exports = plugin;

      reporter-plugin.ts:
        import fs from 'fs';
        import { Reporter } from '@atlaspack/plugin';
        import { someString } from './some-string';
        export default new Reporter({
            async report({ event, options }) {
                if (event.type === 'buildStart') {
                    await options.outputFS.writeFile(options.projectRoot + '/output.txt', 'Hello! ' + someString, 'utf8');
                }
            }
        });

      some-string.ts:
        export const someString = 'something';
        `;

    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS: overlayFS,
      outputFS: overlayFS,
      additionalReporters: [
        {packageName: '@atlaspack/reporter-json', resolveFrom: __filename},
      ],
    });

    await run(b);

    // Tests that the plugin actually loaded properly by validating that it output
    // what it was supposed to output. If `esbuild-register` isn't used, or the resolver
    // doesn't support updating extensions as they change, then the plugin won't work.
    assert(overlayFS.existsSync(path.join(dir, 'output.txt')));
    assert.equal(
      overlayFS.readFileSync(path.join(dir, 'output.txt'), 'utf8'),
      'Hello! something',
    );
  });
});

// Note: In v3, plugins need to use the real filesystem since the worker creates its own NodeFS instance.
describe('TypeScript plugin loading', () => {
  it('should load TypeScript transformer plugins with ESM default export', async () => {
    const dir = path.join(__dirname, 'tmp', 'ts-transformer-plugin');
    await rimraf(dir);
    await inputFS.mkdirp(dir);

    await fsFixture(inputFS, dir)`
      package.json:
        {
          "name": "ts-transformer-test",
          "sideEffects": true,
          "type": "commonjs"
        }

      yarn.lock:

      index.js:
        module.exports = "MARKER_FOR_TRANSFORM";

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.js": ["./transformer-plugin.ts", "..."]
          }
        }

      transformer-plugin.ts:
        import { Transformer } from '@atlaspack/plugin';

        export default new Transformer({
          async transform({ asset }) {
            const code = await asset.getCode();
            if (code.includes('MARKER_FOR_TRANSFORM')) {
              asset.setCode('module.exports = "transformed-by-ts-plugin";');
            }
            return [asset];
          }
        });
    `;

    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS,
      outputFS: inputFS,
      mode: 'production',
      defaultTargetOptions: {
        outputFormat: 'commonjs',
        distDir: path.join(dir, 'dist'),
      },
    });

    let output = await run(b);
    // Handle both CommonJS and ESM module outputs
    assert.equal(output?.default || output, 'transformed-by-ts-plugin');

    await rimraf(dir);
  });

  it('should load TypeScript resolver plugins with CommonJS module.exports', async () => {
    const dir = path.join(__dirname, 'tmp', 'ts-resolver-plugin');
    await rimraf(dir);
    await inputFS.mkdirp(dir);

    await fsFixture(inputFS, dir)`
      package.json:
        {
          "name": "ts-resolver-test",
          "sideEffects": true,
          "type": "commonjs"
        }

      yarn.lock:

      index.js:
        const value = require('virtual-module');
        module.exports = value;

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "resolvers": ["./resolver-plugin.ts", "..."]
        }

      resolver-plugin.ts:
        import { Resolver } from '@atlaspack/plugin';

        module.exports = new Resolver({
          async resolve({ dependency, specifier }) {
            if (specifier === 'virtual-module') {
              return {
                filePath: __dirname + '/virtual-resolved.js'
              };
            }
            return null;
          }
        });

      virtual-resolved.js:
        module.exports = 'resolved-by-ts-resolver';
    `;

    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS,
      outputFS: inputFS,
      mode: 'production',
      defaultTargetOptions: {
        outputFormat: 'commonjs',
        distDir: path.join(dir, 'dist'),
      },
    });

    let output = await run(b);
    // Handle both CommonJS and ESM module outputs
    assert.equal(output?.default || output, 'resolved-by-ts-resolver');

    await rimraf(dir);
  });

  it('should load TypeScript transformer plugins that import other TS files', async () => {
    const dir = path.join(__dirname, 'tmp', 'ts-transformer-with-imports');
    await rimraf(dir);
    await inputFS.mkdirp(dir);

    await fsFixture(inputFS, dir)`
      package.json:
        {
          "name": "ts-transformer-imports-test",
          "sideEffects": true,
          "type": "commonjs"
        }

      yarn.lock:

      index.js:
        module.exports = "MARKER_VALUE";

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.js": ["./transformer-plugin.ts", "..."]
          }
        }

      transformer-plugin.ts:
        import { Transformer } from '@atlaspack/plugin';
        import { replacementValue } from './helpers';

        export default new Transformer({
          async transform({ asset }) {
            const code = await asset.getCode();
            if (code.includes('MARKER_VALUE')) {
              asset.setCode('module.exports = "' + replacementValue + '";');
            }
            return [asset];
          }
        });

      helpers.ts:
        export const replacementValue: string = 'value-from-ts-helper';
    `;

    const b = await bundle(path.join(dir, 'index.js'), {
      inputFS,
      outputFS: inputFS,
      mode: 'production',
      defaultTargetOptions: {
        outputFormat: 'commonjs',
        distDir: path.join(dir, 'dist'),
      },
    });

    let output = await run(b);
    // Handle both CommonJS and ESM module outputs
    assert.equal(output?.default || output, 'value-from-ts-helper');

    await rimraf(dir);
  });
});
