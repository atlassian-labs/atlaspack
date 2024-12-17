// @flow
import assert from 'assert';
import {join} from 'path';
import {
  assertBundles,
  bundle,
  describe,
  fsFixture,
  it,
  overlayFS,
  removeDistDirectory,
  run,
} from '@atlaspack/test-utils';
import type {InitialAtlaspackOptions} from '@atlaspack/types';

describe('bundle-text:', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('inlines and compiles a css bundle', async () => {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        import cssText from 'bundle-text:./styles.css';
        export default cssText;

      img.svg: <svg></svg>

      styles.css:
        body {
          background-color: #000;
        }

        .svg {
          background-image: url('data-url:img.svg');
        }
    `;

    let b = await bundle(join(__dirname, 'index.js'), {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: ['esmodule-helpers.js', 'index.js'],
      },
      {
        type: 'svg',
        assets: ['img.svg'],
      },
      {
        type: 'css',
        assets: ['styles.css'],
      },
    ]);

    let cssBundleContent = (await run(b)).default;

    assert(
      cssBundleContent.startsWith(
        `body {
  background-color: #000;
}

.svg {
  background-image: url("data:image/svg+xml,%3Csvg%3E%3C%2Fsvg%3E");
}`,
      ),
    );

    assert(!cssBundleContent.includes('sourceMappingURL'));
  });

  it.v2('inlines and compiles a html bundle', async () => {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        import html from 'bundle-text:./index.html';
        export default html;

      index.html:
        <p>test</p>
        <script>console.log('test')</script>
    `;

    let b = await bundle(join(__dirname, 'index.js'), {
      inputFS: overlayFS,
    });

    let res = await run(b);
    assert.equal(
      res.default,
      `<p>test</p>\n<script>console.log('test');\n\n</script>`,
    );
  });

  it('inlines and compiles a javascript bundle', async () => {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        import jsText from 'bundle-text:./main.js';
        export default jsText;

      log.js:
        console.log('test');

      main.js:
        import './log';
    `;

    let b = await bundle(join(__dirname, 'index.js'), {
      inputFS: overlayFS,
    });

    let logs = [];
    let res = await run(b, {
      console: {
        log(x) {
          logs.push(x);
        },
      },
    });

    assert(res.default.includes("console.log('test')"));
    assert.deepEqual(logs, []);
  });

  it('inlines and compiles a bundle using a dynamic import', async () => {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        export default import('bundle-text:./styles.css');

      img.svg: <svg></svg>

      styles.css:
         body {
          background-color: #000;
        }

        .svg {
          background-image: url('data-url:img.svg');
        }
    `;

    let b = await bundle(join(__dirname, 'index.js'), {inputFS: overlayFS});

    let promise = (await run(b)).default;
    assert.equal(typeof promise.then, 'function');

    let cssBundleContent = await promise;

    assert(
      cssBundleContent.startsWith(
        `body {
  background-color: #000;
}

.svg {
  background-image: url("data:image/svg+xml,%3Csvg%3E%3C%2Fsvg%3E");
}`,
      ),
    );

    assert(!cssBundleContent.includes('sourceMappingURL'));
  });

  for (const scopeHoist of [false, true]) {
    describe(`when scope hoisting is ${
      scopeHoist ? 'enabled' : 'disabled'
    }`, () => {
      let options: InitialAtlaspackOptions = scopeHoist
        ? {
            defaultTargetOptions: {
              isLibrary: true,
              outputFormat: 'esmodule',
              shouldScopeHoist: true,
            },
          }
        : Object.freeze({});

      it('can be used with an import that points to the same asset', async function () {
        await fsFixture(overlayFS, __dirname)`
          index.js:
            import Test from './main';
            import jsText from 'bundle-text:./main';

            // Workaround bug with exports of symbols with bailouts...
            const t = jsText;
            export { Test, t as jsText };

          main.js:
            export default class Test {};

          package.json: {}
        `;

        let b = await bundle(join(__dirname, 'index.js'), {
          inputFS: overlayFS,
          ...options,
        });

        assertBundles(b, [
          {
            type: 'js',
            assets: [
              'index.js',
              'main.js',
              ...(!scopeHoist ? ['esmodule-helpers.js'] : []),
            ],
          },
          {
            type: 'js',
            assets: scopeHoist
              ? ['main.js']
              : ['main.js', 'esmodule-helpers.js'],
          },
        ]);

        let res = await run(b);
        assert.equal(typeof res.Test, 'function');
        assert.equal(typeof res.jsText, 'string');
      });

      it('can be used with a dynamic import that points to the same asset', async function () {
        await fsFixture(overlayFS, __dirname)`
          index.js:
            import text from 'bundle-text:./main';

            export const lazy = import('./main').then(({ default: Test }) => Test);
            export const jsText = text;

          main.js:
            export default class Test {};
        `;

        let b = await bundle(join(__dirname, 'index.js'), {
          inputFS: overlayFS,
          ...options,
        });

        assertBundles(b, [
          {
            type: 'js',
            assets: scopeHoist
              ? ['index.js']
              : [
                  'index.js',
                  'esmodule-helpers.js',
                  'bundle-url.js',
                  'cacheLoader.js',
                  'js-loader.js',
                ],
          },
          {
            type: 'js',
            assets: ['main.js'],
          },
          {
            type: 'js',
            assets: scopeHoist
              ? ['main.js']
              : ['main.js', 'esmodule-helpers.js'],
          },
        ]);

        let res = await run(b);
        assert.equal(typeof res.lazy, 'object');
        assert.equal(typeof (await res.lazy), 'function');
        assert.equal(typeof res.jsText, 'string');
      });
    });
  }
});
