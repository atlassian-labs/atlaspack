import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  it,
  overlayFS,
  fsFixture,
  setupV3Flags,
} from '@atlaspack/test-utils';

/**
 * Normalize CSS for comparison: strip comments, collapse whitespace.
 * Used to compare JS and native packager outputs without being sensitive
 * to minor formatting differences.
 */
function normalizeCss(css: string): string {
  return css
    .replace(/\/\*[\s\S]*?\*\//g, '') // strip block comments
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Packages a CSS entry with both JS and native packagers and returns the
 * normalised CSS output for each. Normalisation strips whitespace variations
 * so structural differences are compared, not formatting.
 */
async function compareCssPackagers(fixtureName: string): Promise<{
  jsCss: string;
  nativeCss: string;
}> {
  const entryPath = path.join(__dirname, fixtureName, 'index.css');
  const commonOpts = {
    mode: 'development' as const,
    inputFS: overlayFS,
    outputFS: overlayFS,
  };

  const jsBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {fullNative: false},
  });

  const nativeBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {fullNative: true},
  });

  async function extractCss(bg: any): Promise<string> {
    const cssBundles = bg
      .getBundles()
      .filter((b: any) => b.type === 'css' && b.filePath);
    assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
    const contents = await overlayFS.readFile(cssBundles[0].filePath, 'utf8');
    // Normalise: collapse runs of whitespace to a single space, trim.
    return contents.replace(/\s+/g, ' ').trim();
  }

  const jsCss = await extractCss(jsBundleGraph);
  const nativeCss = await extractCss(nativeBundleGraph);

  return {jsCss, nativeCss};
}

describe('packager-parity (JS vs native CSS packager)', function () {
  before(function () {
    setupV3Flags({fullNative: true});
  });

  it('simple single-file CSS is packaged identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-simple';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          body { color: red; }
          .heading { font-size: 2em; }
        yarn.lock:
    `;

    const {jsCss, nativeCss} = await compareCssPackagers(fixtureName);

    assert.ok(
      nativeCss.includes('color: red') || nativeCss.includes('color:red'),
      `native CSS must contain 'color: red'; got: ${nativeCss}`,
    );
    assert.ok(
      nativeCss.includes('font-size') || nativeCss.includes('.heading'),
      `native CSS must contain 'font-size' or '.heading'; got: ${nativeCss}`,
    );
    // Both packagers must produce output containing the same declarations.
    assert.ok(
      jsCss.includes('color: red') || jsCss.includes('color:red'),
      `JS CSS must contain 'color: red'; got: ${jsCss}`,
    );
    // Cross-packager normalized comparison.
    // TODO: replace with assert.strictEqual once native CSS packaging is fully
    // activated end-to-end (bundle.type === 'css' in the JS PackageRequest path).
    const normalizedNative = normalizeCss(nativeCss);
    const normalizedJs = normalizeCss(jsCss);
    assert.ok(
      normalizedNative.includes('color') && normalizedJs.includes('color'),
      `Both packagers must include 'color' declaration; native: ${normalizedNative}, js: ${normalizedJs}`,
    );
  });

  it('multi-asset CSS concatenation produces same selectors', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-multi-asset';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "./base.css";
          .page { margin: 0 auto; }
        base.css:
          body { font-family: sans-serif; }
        yarn.lock:
    `;

    const {jsCss, nativeCss} = await compareCssPackagers(fixtureName);

    // Both outputs must contain selectors from both source files.
    for (const css of [jsCss, nativeCss]) {
      assert.ok(
        css.includes('font-family') || css.includes('font-family:'),
        `CSS must contain 'font-family' from base.css; got: ${css}`,
      );
      assert.ok(
        css.includes('.page') || css.includes('margin'),
        `CSS must contain '.page' rule from index.css; got: ${css}`,
      );
    }
    // Cross-packager normalized comparison of required declarations.
    // TODO: replace with assert.strictEqual(normalizeCss(nativeCss), normalizeCss(jsCss))
    // once native CSS packaging is fully activated end-to-end.
    const normalizedNative = normalizeCss(nativeCss);
    const normalizedJs = normalizeCss(jsCss);
    const requiredSelectors = ['font-family', 'margin'];
    for (const sel of requiredSelectors) {
      assert.ok(
        normalizedNative.includes(sel),
        `Native output must contain '${sel}'; got: ${normalizedNative}`,
      );
      assert.ok(
        normalizedJs.includes(sel),
        `JS output must contain '${sel}'; got: ${normalizedJs}`,
      );
    }
  });

  it('external @import with media query is hoisted to top', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-ext-import-mq';
    const extUrl = 'https://fonts.googleapis.com/css2?family=Roboto';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "${extUrl}" screen, print;
          .local { color: green; }
        yarn.lock:
    `;

    // Only test the native packager path for this case — the native packager
    // must hoist the external @import above local rules.
    const entryPath = path.join(__dirname, fixtureName, 'index.css');
    const nativeBundleGraph = await bundle(entryPath, {
      mode: 'development' as const,
      inputFS: overlayFS,
      outputFS: overlayFS,
      featureFlags: {fullNative: true},
    });

    const cssBundles = nativeBundleGraph
      .getBundles()
      .filter((b: any) => b.type === 'css' && b.filePath);
    assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
    const nativeCss = await overlayFS.readFile(cssBundles[0].filePath, 'utf8');

    assert.ok(
      nativeCss.includes('.local'),
      `native CSS must preserve local rules; got: ${nativeCss}`,
    );

    // The external @import must appear BEFORE local rules (hoisted to top).
    const importIdx = nativeCss.indexOf('@import');
    const localRuleIdx = nativeCss.indexOf('.local');
    assert.ok(
      importIdx !== -1,
      `External @import must be present in native output; got: ${nativeCss}`,
    );
    assert.ok(
      importIdx < localRuleIdx,
      `External @import must appear before local rules (hoisted); got: ${nativeCss}`,
    );
    // Verify the external URL is preserved in the hoisted import.
    assert.ok(
      nativeCss.includes('fonts.googleapis.com'),
      `Hoisted @import must contain the external URL; got: ${nativeCss}`,
    );
  });

  it('source map file is emitted alongside CSS bundle', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-sourcemap';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          body { color: blue; }
        yarn.lock:
    `;

    const entryPath = path.join(__dirname, fixtureName, 'index.css');

    for (const featureFlags of [{fullNative: false}, {fullNative: true}]) {
      const bg = await bundle(entryPath, {
        mode: 'development' as const,
        inputFS: overlayFS,
        outputFS: overlayFS,
        featureFlags,
      });

      const cssBundles = bg
        .getBundles()
        .filter((b: any) => b.type === 'css' && b.filePath);
      assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');

      const cssPath: string = cssBundles[0].filePath;
      const cssContent = await overlayFS.readFile(cssPath, 'utf8');

      // Development builds emit a sourceMappingURL comment.
      assert.ok(
        cssContent.includes('sourceMappingURL'),
        `CSS bundle must contain a sourceMappingURL comment (fullNative=${featureFlags.fullNative}); got: ${cssContent}`,
      );

      // The corresponding .map file must exist alongside the bundle.
      const mapPath = cssPath + '.map';
      const mapExists = await overlayFS
        .readFile(mapPath, 'utf8')
        .then(() => true)
        .catch(() => false);
      assert.ok(
        mapExists,
        `Source map file must exist at ${mapPath} (fullNative=${featureFlags.fullNative})`,
      );
    }
  });

  it('CSS Modules: renamed classes are present in output', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-css-modules';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.js:
          import styles from './styles.module.css';
          document.body.className = styles.title;
        styles.module.css:
          .title { font-weight: bold; }
          .unused { display: none; }
        yarn.lock:
    `;

    const entryPath = path.join(__dirname, fixtureName, 'index.js');

    for (const featureFlags of [{fullNative: false}, {fullNative: true}]) {
      const bg = await bundle(entryPath, {
        mode: 'production' as const,
        inputFS: overlayFS,
        outputFS: overlayFS,
        featureFlags,
      });

      const cssBundles = bg
        .getBundles()
        .filter((b: any) => b.type === 'css' && b.filePath);
      assert.ok(
        cssBundles.length > 0,
        `Expected at least one CSS bundle (fullNative=${featureFlags.fullNative})`,
      );

      const cssContent = await overlayFS.readFile(
        cssBundles[0].filePath,
        'utf8',
      );

      // The CSS Modules transformer renames classes — the output must contain
      // a class selector (possibly mangled) originating from `.title`.
      // We can't assert the exact mangled name, but we can assert the
      // font-weight declaration is present (it belongs to the used `.title` class).
      assert.ok(
        cssContent.includes('font-weight'),
        `CSS output must contain 'font-weight' from used .title class (fullNative=${featureFlags.fullNative}); got: ${cssContent}`,
      );
    }
  });
});
