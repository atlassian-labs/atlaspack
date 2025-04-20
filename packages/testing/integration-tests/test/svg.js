// @flow
import assert from 'assert';
import {
  assertBundles,
  bundle,
  describe,
  distDir,
  it,
  outputFS,
} from '@atlaspack/test-utils';
import path from 'path';

describe('svg', function () {
  it.v2('should support bundling SVG', async () => {
    let b = await bundle(path.join(__dirname, '/integration/svg/circle.svg'));

    assertBundles(b, [
      {
        name: 'circle.svg',
        assets: ['circle.svg'],
      },
      {
        name: 'other1.html',
        assets: ['other1.html'],
      },
      {
        type: 'svg',
        assets: ['square.svg'],
      },
      {
        name: 'other2.html',
        assets: ['other2.html'],
      },
      {
        type: 'svg',
        assets: ['path.svg'],
      },
      {
        type: 'svg',
        assets: ['gradient.svg'],
      },
      {
        type: 'js',
        assets: ['script.js'],
      },
      {
        type: 'js',
        assets: ['module.js', 'script.js'],
      },
      {
        type: 'css',
        assets: ['style.css'],
      },
    ]);

    let svgBundle = b.getBundles().find((b) => b.type === 'svg');
    if (!svgBundle) return assert.fail();

    let file = await outputFS.readFile(svgBundle.filePath, 'utf-8');
    assert(file.includes('<a href="/other1.html">'));
    assert(file.includes('<use href="#circle"'));

    let squareBundle = b.getBundles().find((b) => b.name.startsWith('square'));
    if (!squareBundle) return assert.fail();

    assert(
      file.includes(
        `<use xlink:href="/${path.basename(squareBundle.filePath)}#square"`,
      ),
    );

    let gradientBundle = b
      .getBundles()
      .find((b) => b.name.startsWith('gradient'));
    if (!gradientBundle) return assert.fail();

    assert(
      file.includes(
        `fill="url('/${path.basename(gradientBundle.filePath)}#myGradient')"`,
      ),
    );

    let scriptBundle = b
      .getBundles()
      .find((b) => b.type === 'js' && b.env.sourceType === 'script');
    if (!scriptBundle) return assert.fail();

    assert(
      file.includes(
        `<script xlink:href="/${path.basename(scriptBundle.filePath)}"`,
      ),
    );

    let moduleBundle = b
      .getBundles()
      .find((b) => b.type === 'js' && b.env.sourceType === 'module');
    if (!moduleBundle) return assert.fail();

    assert(
      file.includes(`<script href="/${path.basename(moduleBundle.filePath)}"`),
    );

    let cssBundle = b.getBundles().find((b) => b.type === 'css');
    if (!cssBundle) return assert.fail();

    assert(
      file.includes(
        `<?xml-stylesheet href="/${path.basename(cssBundle.filePath)}"?>`,
      ),
    );
  });

  it.v2('should minify SVG bundles', async function () {
    let b = await bundle(path.join(__dirname, '/integration/svg/circle.svg'), {
      defaultTargetOptions: {
        shouldOptimize: true,
      },
    });

    let svgBundle = b.getBundles().find((b) => b.type === 'svg');
    if (!svgBundle) return assert.fail();

    let file = await outputFS.readFile(svgBundle.filePath, 'utf-8');
    assert(!file.includes('comment'));
  });

  it.v2('support SVGO config files', async function () {
    let b = await bundle(
      path.join(__dirname, '/integration/svgo-config/index.html'),
      {
        defaultTargetOptions: {
          shouldOptimize: true,
        },
      },
    );

    let svgBundle = b.getBundles().find((b) => b.type === 'svg');
    if (!svgBundle) return assert.fail();

    let file = await outputFS.readFile(svgBundle.filePath, 'utf-8');
    assert(!file.includes('inkscape'));
    assert(file.includes('comment'));
  });

  it.v2(
    'should detect xml-stylesheet processing instructions',
    async function () {
      let b = await bundle(
        path.join(__dirname, '/integration/svg-xml-stylesheet/img.svg'),
      );

      assertBundles(b, [
        {
          name: 'img.svg',
          assets: ['img.svg'],
        },
        {
          type: 'css',
          assets: ['style1.css'],
        },
        {
          type: 'css',
          assets: ['style3.css'],
        },
      ]);

      let svgBundle = b.getBundles().find((b) => b.type === 'svg');
      if (!svgBundle) return assert.fail();

      let file = await outputFS.readFile(svgBundle.filePath, 'utf-8');

      assert(file.includes('<?xml-stylesheet'));
      assert(file.includes('<?xml-not-a-stylesheet'));
    },
  );

  it.v2('should handle inline CSS with @imports', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/svg-inline-css-import/img.svg'),
    );

    assertBundles(b, [
      {
        type: 'css',
        assets: ['img.svg', 'test.css'],
      },
      {
        type: 'css',
        assets: ['img.svg'],
      },
      {
        name: 'img.svg',
        assets: ['img.svg'],
      },
      {
        type: 'svg',
        assets: ['gradient.svg'],
      },
      {
        type: 'js',
        assets: ['img.svg', 'script.js'],
      },
    ]);

    const svg = await outputFS.readFile(path.join(distDir, 'img.svg'), 'utf8');

    assert(!svg.includes('@import'));
    assert(svg.includes(':root {\n  fill: red;\n}'));

    let gradientBundle = b
      .getBundles()
      .find((b) => b.name.startsWith('gradient'));
    if (!gradientBundle) return assert.fail();

    assert(
      svg.includes(
        `"fill: url(&quot;${path.basename(
          gradientBundle.filePath,
        )}#myGradient&quot;)`,
      ),
    );
    assert(svg.includes('<script>'));
    assert(svg.includes(`console.log('script')`));
    assert(!svg.includes('import '));
  });

  it.v2('should process inline styles using lang', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/svg-inline-sass/img.svg'),
      {
        defaultTargetOptions: {
          shouldOptimize: true,
        },
      },
    );

    assertBundles(b, [
      {
        type: 'css',
        assets: ['img.svg'],
      },
      {
        name: 'img.svg',
        assets: ['img.svg'],
      },
    ]);

    const svg = await outputFS.readFile(path.join(distDir, 'img.svg'), 'utf8');

    assert(svg.includes('<style>:root{fill:red}</style>'));
  });

  it('should be in separate bundles', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/svg-multiple/index.js'),
    );

    assertBundles(b, [
      {
        assets: ['index.js', 'bundle-url.js'],
      },
      {
        assets: ['circle.svg'],
      },
      {
        assets: ['square.svg'],
      },
    ]);
  });
});
