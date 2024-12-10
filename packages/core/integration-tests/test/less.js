// @flow
import assert from 'assert';
import {join} from 'path';
import {
  assertBundles,
  bundle,
  describe,
  distDir,
  fsFixture,
  it,
  outputFS,
  overlayFS,
  run,
} from '@atlaspack/test-utils';
import {md} from '@atlaspack/diagnostic';

describe.v2('less', function () {
  it('should support requiring less files', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.js:
        require('./index.less');

      index.less:
        @base: #f938ab;

        .index {
          color: @base;
        }
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js'],
      },
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(css.includes('.index'));
  });

  it('should support less imports', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      a.less:
        .a { color: red }

      b.less:
        .b { color: orange }

      c.less:
        .c { color: yellow }

      d.less:
        .d { color: green }

      index.less:
        @import './a.less';
        @import 'b.less';
        @import './c';
        @import 'd';
    `;

    let b = await bundle('index.less', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(css.includes('.a'));
    assert(css.includes('.b'));
    assert(css.includes('.c'));
    assert(css.includes('.d'));
  });

  it('should support advanced less imports', async function () {
    await fsFixture(overlayFS)`
      nested/externals.less:
        @import '~/node_modules/explicit-external-less/a.less';
        @import 'external-less';
        @import 'external-less/a.less';
        @import 'external-less-with-main';

      node_modules/explicit-external-less/a.less:
        .explicit-external-a {
          background: red;
        }

      node_modules/external-less/a.less:
        .external-a {
          background: red;
        }

      node_modules/external-less/index.less:
        .external-index {
          color: red;
        }

      node_modules/external-less/package.json: {}

      node_modules/external-less-with-main/main.less:
        .external-with-main {
          color: red;
        }

      node_modules/external-less-with-main/package.json:
        {
          "main": "main.less"
        }

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      a.less:
        .a {
          color: red;
        }

      index.js:
        require('~/index.less');

      index.less:
        @import '~/a.less';
        @import './nested/externals.less';
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js'],
      },
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');

    assert(css.includes('.a'));
    assert(css.includes('.external-index'));
    assert(css.includes('.external-a'));
    assert(css.includes('.external-with-main'));
    assert(css.includes('.explicit-external-a'));
  });

  it('should support requiring empty less files', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.js:
        require('./index.less');

      index.less:
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js'],
      },
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert.equal(css.trim(), '/*# sourceMappingURL=index.css.map */');
  });

  it('should support linking to assets with url() from less', async function () {
    await fsFixture(overlayFS)`
      fonts/test.woff2: test

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.less:
        @font-face {
          font-family: "Test";
          src: url("./fonts/test.woff2") format("woff2");
        }

        .index {
          background: url("http://google.com");
        }
    `;

    let b = await bundle('index.less', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.css',
        assets: ['index.less'],
      },
      {
        type: 'woff2',
        assets: ['test.woff2'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(/url\("?test\.[0-9a-f]+\.woff2"?\)/.test(css));
    assert(/url\("?http:\/\/google.com"?\)/.test(css));
    assert(css.includes('.index'));

    assert(
      await outputFS.exists(
        join(distDir, css.match(/url\("?(test\.[0-9a-f]+\.woff2)"?\)/)[1]),
      ),
    );
  });

  it('should support less url rewrites', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      nested/a.less:
        @font-face {
          font-family: "A";
          src: url("./a.woff2") format("woff2");
        }

        .a {
          font-family: "A";
        }

      nested/a.woff2: test

      node_modules/library/b.less:
        @font-face {
          font-family: "B";
          src: url("./b.woff2") format("woff2");
        }

        .b {
          font-family: "B";
        }

      node_modules/library/b.woff2: test

      index.less:
        @import "./nested/a.less";
        @import "./node_modules/library/b.less";
    `;

    let b = await bundle('index.less', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.css',
        assets: ['index.less'],
      },
      {
        type: 'woff2',
        assets: ['a.woff2'],
      },
      {
        type: 'woff2',
        assets: ['b.woff2'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(css.includes('.a'));
    assert(css.includes('.b'));
  });

  it('should support css modules in less', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      img.svg:

      index.js:
        var map = require('./index.module.less');

        module.exports = function () {
          return map.index;
        };

      index.module.less:
        @base: #f938ab;

        .index {
          color: @base;
          background-image: url('img.svg');
        }
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js', 'index.module.less'],
      },
      {
        name: 'index.css',
        assets: ['index.module.less'],
      },
      {
        assets: ['img.svg'],
      },
    ]);

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert(output().endsWith('_index'));

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(/\.[_0-9a-zA-Z]+_index/.test(css));
  });

  it('should throw an exception when using webpack syntax', async function () {
    await fsFixture(overlayFS)`
      node_modules/library/styles.less:

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.less:
        @import '~library/style.less';
    `;

    // $FlowFixMe
    await assert.rejects(() => bundle('index.less', {inputFS: overlayFS}), {
      message: md`The @import path "${'~library/style.less'}" is using webpack specific syntax, which isn't supported by Parcel.\n\nTo @import files from ${'node_modules'}, use "${'library/style.less'}"`,
    });
  });

  it('should support configuring less include paths', async function () {
    await fsFixture(overlayFS)`
      include-path/a.less:
        .a {
          color: red;
        }

      node_modules/library/b.less:
        .b {
          color: red;
        }

      index.js:
        require('./index.less');

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      .lessrc.js:
        const { join } = require('path');

        module.exports = {
          paths: [
            join(__dirname, 'include-path'),
            join(__dirname, 'node_modules', 'library')
          ]
        };

      index.less:
        @import 'a.less';
        @import 'b.less';
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js'],
      },
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(css.includes('.a'));
    assert(css.includes('.b'));
  });

  it('should ignore url() with IE behavior specifiers', async function () {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.less:
        .index {
          behavior: url(#default#VML);
        }
    `;

    let b = await bundle('index.less', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');

    assert(css.includes('url("#default#VML")'));
  });

  it('preserves quotes around data urls that require them', async () => {
    await fsFixture(overlayFS)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.less:
        .index {
          // Note the literal space after "xml"
          background: url("data:image/svg+xml,%3C%3Fxml version%3D%221.0%22%3F%3E%3Csvg%3E%3C%2Fsvg%3E");
        }
    `;

    let b = await bundle('index.less', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.css',
        assets: ['index.less'],
      },
    ]);

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(
      css.includes(
        // Note the literal space after "xml"
        'background: url("data:image/svg+xml,%3C%3Fxml version%3D%221.0%22%3F%3E%3Csvg%3E%3C%2Fsvg%3E")',
      ),
    );
  });

  it('should support package exports style condition', async function () {
    await fsFixture(overlayFS)`
      node_modules/foo/a.less:
        .a {
          color: red;
        }

      node_modules/foo/package.json:
        {
          "exports": {
            "style": "./a.less"
          }
        }

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "transformers": {
            "*.less": ["@atlaspack/transformer-less"]
          }
        }

      index.less:
        @import "foo";

      package.json:
        {
          "@atlaspack/resolver-default": {
            "packageExports": true
          }
        }

      yarn.lock: {}
    `;

    await bundle('index.less', {inputFS: overlayFS});

    let css = await outputFS.readFile(join(distDir, 'index.css'), 'utf8');
    assert(css.includes('.a'));
  });
});
