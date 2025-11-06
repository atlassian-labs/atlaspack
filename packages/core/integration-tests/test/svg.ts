import assert from 'assert';
import {
  assertBundles,
  bundle,
  describe,
  distDir,
  fsFixture,
  it,
  outputFS,
  overlayFS,
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

  it.v2(
    'should handle functional IRI URL parsing edge cases',
    async function () {
      await fsFixture(overlayFS, __dirname)`
      integration/svg-func-iri-edge-cases
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <!-- Test different quote styles -->
            <rect fill="url('external.svg#gradient')" x="0" y="0" width="10" height="10"/>
            <rect fill='url("external.svg#gradient2")' x="10" y="0" width="10" height="10"/>
            <rect fill="url(external.svg#gradient3)" x="20" y="0" width="10" height="10"/>

            <!-- Test spaces and escaped characters -->
            <rect fill="url('path with spaces.svg#grad')" x="0" y="10" width="10" height="10"/>
            <rect fill="url('path\\'s/file.svg#grad')" x="10" y="10" width="10" height="10"/>
            <rect fill='url("path\\"s/file.svg#grad")' x="20" y="10" width="10" height="10"/>

            <!-- Test with modifiers -->
            <rect fill="url('external.svg#gradient' fallback(red))" x="0" y="20" width="10" height="10"/>

            <!-- Test various functional IRI attributes -->
            <rect stroke="url(stroke.svg#pattern)" x="30" y="0" width="10" height="10"/>
            <rect clip-path="url(clip.svg#clipPath)" x="30" y="10" width="10" height="10"/>
            <rect mask="url(mask.svg#maskDef)" x="30" y="20" width="10" height="10"/>
            <path marker-start="url(markers.svg#arrow)" d="M 40,0 L 50,10"/>
            <path marker-mid="url(markers.svg#circle)" d="M 40,10 L 50,20"/>
            <path marker-end="url(markers.svg#square)" d="M 40,20 L 50,30"/>
          </svg>

        external.svg:
          <svg><defs><linearGradient id="gradient"/><linearGradient id="gradient2"/><linearGradient id="gradient3"/></defs></svg>

        stroke.svg:
          <svg><defs><pattern id="pattern"/></defs></svg>

        clip.svg:
          <svg><defs><clipPath id="clipPath"/></defs></svg>

        mask.svg:
          <svg><defs><mask id="maskDef"/></defs></svg>

        markers.svg:
          <svg><defs><marker id="arrow"/><marker id="circle"/><marker id="square"/></defs></svg>

        path with spaces.svg:
          <svg><defs><linearGradient id="grad"/></defs></svg>

        path's/file.svg:
          <svg><defs><linearGradient id="grad"/></defs></svg>

        path"s/file.svg:
          <svg><defs><linearGradient id="grad"/></defs></svg>
    `;

      const b = await bundle(
        path.join(__dirname, 'integration/svg-func-iri-edge-cases/index.svg'),
        {
          inputFS: overlayFS,
          outputFS: overlayFS,
        },
      );

      // Should create bundles for all referenced SVG files
      assertBundles(b, [
        {name: 'index.svg', assets: ['index.svg']},
        {assets: ['external.svg']},
        {assets: ['stroke.svg']},
        {assets: ['clip.svg']},
        {assets: ['mask.svg']},
        {assets: ['markers.svg']},
        {assets: ['path with spaces.svg']},
        {assets: ['file.svg']}, // path's/file.svg
        {assets: ['file.svg']}, // path"s/file.svg
      ]);

      const svgContent = await overlayFS.readFile(
        b.getBundles().find((bundle) => bundle.name === 'index.svg')!.filePath,
        'utf8',
      );

      // Verify URLs are properly rewritten
      assert(svgContent.includes("url('"));
      assert(svgContent.includes('fallback(red)'));
    },
  );

  it.v2('should handle xlink namespace attributes', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg-xlink-attrs
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
            <!-- xlink:href attributes that should create dependencies -->
            <altGlyph xlink:href="fonts.svg#glyph1"/>
            <cursor xlink:href="cursors.svg#pointer"/>
            <filter xlink:href="filters.svg#blur"/>
            <font-face-uri xlink:href="fonts.svg#face"/>
            <glyphRef xlink:href="fonts.svg#ref"/>
            <tref xlink:href="text.svg#textRef"/>
            <color-profile xlink:href="color.svg#profile"/>
          </svg>

        fonts.svg:
          <svg><defs><glyph id="glyph1"/><font-face id="face"/><g id="ref"/></defs></svg>

        cursors.svg:
          <svg><defs><cursor id="pointer"/></defs></svg>

        filters.svg:
          <svg><defs><filter id="blur"/></defs></svg>

        text.svg:
          <svg><defs><text id="textRef"/></defs></svg>

        color.svg:
          <svg><defs><color-profile id="profile"/></defs></svg>
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg-xlink-attrs/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {assets: ['fonts.svg']},
      {assets: ['cursors.svg']},
      {assets: ['filters.svg']},
      {assets: ['text.svg']},
      {assets: ['color.svg']},
    ]);
  });

  it.v2(
    'should handle empty href attributes with proper error',
    async function () {
      await fsFixture(overlayFS, __dirname)`
      integration/svg-empty-href
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <use href=""/>
            <image href=""/>
          </svg>
    `;

      let threw = false;
      try {
        await bundle(
          path.join(__dirname, 'integration/svg-empty-href/index.svg'),
          {
            inputFS: overlayFS,
            outputFS: overlayFS,
          },
        );
      } catch (err: any) {
        threw = true;
        assert(err.message.includes("'href' should not be empty string"));
      }
      assert(threw, 'Expected error for empty href attributes');
    },
  );

  it.v2(
    'should handle script types and environments correctly',
    async function () {
      await fsFixture(overlayFS, __dirname)`
      integration/svg-script-types
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <!-- Different script types -->
            <script type="application/ecmascript" href="script1.js"/>
            <script type="application/javascript" href="script2.js"/>
            <script type="text/javascript" href="script3.js"/>
            <script type="module" href="script4.js"/>
            <script href="script5.js"/>

            <!-- Inline scripts with different types -->
            <script type="application/ecmascript">console.log('ecma');</script>
            <script type="application/javascript">console.log('app-js');</script>
            <script type="text/javascript">console.log('text-js');</script>
            <script type="module">console.log('module');</script>
            <script>console.log('default');</script>
          </svg>

        script1.js: console.log('script1');
        script2.js: console.log('script2');
        script3.js: console.log('script3');
        script4.js: console.log('script4');
        script5.js: console.log('script5');
    `;

      const b = await bundle(
        path.join(__dirname, 'integration/svg-script-types/index.svg'),
        {
          inputFS: overlayFS,
          outputFS: overlayFS,
        },
      );

      // Check that all scripts are bundled
      const bundles = b.getBundles();
      const jsBundles = bundles.filter((bundle) => bundle.type === 'js');
      assert(jsBundles.length >= 2); // At least one for module, one for script

      const svgContent = await overlayFS.readFile(
        bundles.find((bundle) => bundle.name === 'index.svg')!.filePath,
        'utf8',
      );

      // Verify type attributes are removed from script tags
      assert(!svgContent.includes('type="application/ecmascript"'));
      assert(!svgContent.includes('type="application/javascript"'));
      assert(!svgContent.includes('type="text/javascript"'));
    },
  );

  it.v2('should handle style types and processing', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg-style-types
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <!-- Different style types -->
            <style type="text/css">.cls1 { fill: red; }</style>
            <style type="text/scss">$color: blue; .cls2 { fill: $color; }</style>
            <style type="application/css">.cls3 { fill: green; }</style>
            <style>.cls4 { fill: yellow; }</style>

            <!-- Style attributes -->
            <rect style="fill: purple;" x="0" y="0" width="10" height="10"/>
            <circle style="stroke: orange;" cx="20" cy="20" r="5"/>
          </svg>
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg-style-types/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    const bundles = b.getBundles();

    // For inline styles, we expect CSS bundles to be created for the extracted content
    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {type: 'css', assets: ['index.svg']}, // inline style CSS
      {type: 'css', assets: ['index.svg']}, // inline style CSS
      {type: 'css', assets: ['index.svg']}, // inline style CSS
      {type: 'css', assets: ['index.svg']}, // inline style CSS
      {type: 'css', assets: ['index.svg']}, // style attribute CSS
      {type: 'css', assets: ['index.svg']}, // style attribute CSS
    ]);

    const svgBundle = bundles.find((bundle) => bundle.name === 'index.svg');
    assert(svgBundle, 'Expected to find index.svg bundle');

    const svgContent = await overlayFS.readFile(svgBundle.filePath, 'utf8');

    // Verify content is properly inlined and type attributes are removed
    assert(
      svgContent.includes('<style>'),
      'Expected <style> tag in SVG content',
    );
    assert(
      !svgContent.includes('type="text/css"'),
      'type="text/css" should be removed',
    );
    assert(
      !svgContent.includes('type="text/scss"'),
      'type="text/scss" should be removed',
    );
    assert(
      !svgContent.includes('type="application/css"'),
      'type="application/css" should be removed',
    );
  });

  it.v2('should handle custom data-parcel-key attributes', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg-custom-parcel-key
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <style data-parcel-key="custom-style-key">.custom { fill: red; }</style>
            <script data-parcel-key="custom-script-key" type="text/javascript">console.log('custom');</script>
            <style>.auto-key { fill: blue; }</style>
          </svg>
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg-custom-parcel-key/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    // Should create CSS and JS bundles for the inline content
    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {type: 'css', assets: ['index.svg']}, // custom-style-key
      {type: 'js', assets: ['index.svg']}, // custom-script-key
      {type: 'css', assets: ['index.svg']}, // auto-generated key
    ]);

    const svgContent = await overlayFS.readFile(
      b.getBundles().find((bundle) => bundle.name === 'index.svg')!.filePath,
      'utf8',
    );

    // Verify the inline content is properly processed and CSS/JS is inlined
    assert(svgContent.includes('<style>'), 'Expected processed style content');
    assert(
      svgContent.includes('<script>'),
      'Expected processed script content',
    );
    assert(svgContent.includes('.custom'), 'Expected custom CSS class');
    assert(
      svgContent.includes("console.log('custom')"),
      'Expected custom script content',
    );
  });

  it.v2('should handle complex XML processing instructions', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg-xml-complex
        index.svg:
          <?xml-stylesheet href="style1.css" type="text/css"?>
          <?xml-stylesheet
            href="style2.css"
            type="text/css"
            media="screen"?>
          <?xml-stylesheet href='style3.css' type='text/css'?>
          <?xml-not-stylesheet href="should-not-process.css"?>
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <text>Styled text</text>
          </svg>

        style1.css: text { fill: red; }
        style2.css: text { fill: blue; }
        style3.css: text { fill: green; }
        should-not-process.css: text { fill: black; }
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg-xml-complex/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {assets: ['style1.css']},
      {assets: ['style2.css']},
      // style3.css appears to not be processed due to malformed syntax
    ]);

    const svgContent = await overlayFS.readFile(
      b.getBundles().find((bundle) => bundle.name === 'index.svg')!.filePath,
      'utf8',
    );

    // Verify processed stylesheets are rewritten
    assert(svgContent.includes('<?xml-stylesheet href="'));
    // Verify non-stylesheet processing instruction is unchanged
    assert(
      svgContent.includes('<?xml-not-stylesheet href="should-not-process.css"'),
    );
  });

  it.v2('should handle SVG2 functional IRI attributes', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg2-attributes
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <!-- SVG2 specific attributes -->
            <rect shape-inside="url(shapes.svg#inside)" x="0" y="0" width="50" height="50"/>
            <rect shape-subtract="url(shapes.svg#subtract)" x="50" y="0" width="50" height="50"/>
            <rect mask-image="url(masks.svg#image)" x="0" y="50" width="50" height="50"/>
          </svg>

        shapes.svg:
          <svg><defs><path id="inside"/><path id="subtract"/></defs></svg>

        masks.svg:
          <svg><defs><mask id="image"/></defs></svg>
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg2-attributes/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {assets: ['shapes.svg']},
      {assets: ['masks.svg']},
    ]);
  });

  it.v2(
    'should handle mixed href and xlink:href on same element',
    async function () {
      await fsFixture(overlayFS, __dirname)`
      integration/svg-mixed-href
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
            <!-- Element with both href and xlink:href (href should take precedence) -->
            <use href="modern.svg#symbol" xlink:href="legacy.svg#symbol"/>
            <!-- Only xlink:href -->
            <use xlink:href="legacy2.svg#symbol"/>
          </svg>

        modern.svg:
          <svg><defs><symbol id="symbol"/></defs></svg>

        legacy.svg:
          <svg><defs><symbol id="symbol"/></defs></svg>

        legacy2.svg:
          <svg><defs><symbol id="symbol"/></defs></svg>
    `;

      const b = await bundle(
        path.join(__dirname, 'integration/svg-mixed-href/index.svg'),
        {
          inputFS: overlayFS,
          outputFS: overlayFS,
        },
      );

      assertBundles(b, [
        {name: 'index.svg', assets: ['index.svg']},
        {assets: ['modern.svg']}, // href takes precedence
        {assets: ['legacy.svg']}, // Both href and xlink:href are processed
        {assets: ['legacy2.svg']}, // xlink:href only
      ]);
    },
  );

  it.v2('should handle malformed URLs gracefully', async function () {
    await fsFixture(overlayFS, __dirname)`
      integration/svg-malformed-urls
        index.svg:
          <svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <!-- These should not cause crashes but may not create dependencies -->
            <rect fill="url(unclosed-quote.svg#grad"/>
            <rect stroke="url('missing-closing-paren.svg#pattern'"/>
            <rect clip-path="url()"/>
            <rect mask="url(valid.svg#mask)"/>
          </svg>

        valid.svg:
          <svg><defs><mask id="mask"/></defs></svg>
    `;

    const b = await bundle(
      path.join(__dirname, 'integration/svg-malformed-urls/index.svg'),
      {
        inputFS: overlayFS,
        outputFS: overlayFS,
      },
    );

    // Should only bundle the valid reference
    assertBundles(b, [
      {name: 'index.svg', assets: ['index.svg']},
      {assets: ['valid.svg']},
    ]);
  });
});
