import assert from 'assert';
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

describe.v2('bundle-text:', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('inlines and compiles a css bundle', async () => {
    await fsFixture(overlayFS)`
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

    let b = await bundle('index.js', {inputFS: overlayFS});

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

  it('inlines and compiles a html bundle', async () => {
    await fsFixture(overlayFS)`
      index.js:
        import html from 'bundle-text:./index.html';
        export default html;

      index.html:
        <p>test</p>
        <script>console.log('test')</script>
    `;

    let b = await bundle('index.js', {
      inputFS: overlayFS,
    });

    let res = await run(b);
    assert.equal(
      res.default,
      `<p>test</p>\n<script>console.log('test');\n\n</script>`,
    );
  });

  it('inlines and compiles a javascript bundle', async () => {
    await fsFixture(overlayFS)`
      index.js:
        import jsText from 'bundle-text:./main.js';
        export default jsText;

      log.js:
        console.log('test');

      main.js:
        import './log';
    `;

    let b = await bundle('index.js', {
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
    await fsFixture(overlayFS)`
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

    let b = await bundle('index.js', {inputFS: overlayFS});

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
});
