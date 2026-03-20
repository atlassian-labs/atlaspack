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
 * Normalize CSS for comparison: strip block comments, collapse whitespace.
 * Used to compare JS and native packager outputs without sensitivity to minor
 * formatting differences (e.g. extra newlines, whitespace inside rules).
 */
function normalizeCss(css: string): string {
  return css
    .replace(/\/\*[\s\S]*?\*\//g, '') // strip block comments
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Reads the first CSS bundle from a bundle graph and returns its normalised
 * content. Normalisation strips block comments and collapses whitespace so
 * comparisons are not sensitive to formatting differences.
 */
async function extractCssBundleContent(bg: any): Promise<string> {
  const cssBundles = bg
    .getBundles()
    .filter((b: any) => b.type === 'css' && b.filePath);
  assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
  const raw = await overlayFS.readFile(cssBundles[0].filePath, 'utf8');
  return normalizeCss(raw);
}

/**
 * Packages a CSS entry with both JS and native packagers and returns the
 * normalised CSS output for each.
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

  const jsCss = await extractCssBundleContent(jsBundleGraph);
  const nativeCss = await extractCssBundleContent(nativeBundleGraph);

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

    // Both packagers must produce structurally identical normalised output.
    assert.strictEqual(
      nativeCss,
      jsCss,
      `Native and JS packagers must produce identical normalised CSS.\nNative: ${nativeCss}\nJS:     ${jsCss}`,
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

    // Both packagers must concatenate assets and produce identical output.
    assert.strictEqual(
      nativeCss,
      jsCss,
      `Native and JS packagers must produce identical normalised CSS for multi-asset bundle.\nNative: ${nativeCss}\nJS:     ${jsCss}`,
    );
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

    // Only test the native packager path — the native packager must hoist the
    // external @import above local rules.
    const entryPath = path.join(__dirname, fixtureName, 'index.css');
    const nativeBundleGraph = await bundle(entryPath, {
      mode: 'development' as const,
      inputFS: overlayFS,
      outputFS: overlayFS,
      featureFlags: {fullNative: true},
    });

    const nativeCss = await overlayFS.readFile(
      nativeBundleGraph
        .getBundles()
        .find((b: any) => b.type === 'css' && b.filePath).filePath,
      'utf8',
    );

    assert.ok(
      nativeCss.includes('.local'),
      `native CSS must preserve local rules; got: ${nativeCss}`,
    );

    // The external @import must appear BEFORE local rules (hoisted to top).
    const importIdx = nativeCss.indexOf('@import');
    const localRuleIdx = nativeCss.indexOf('.local');
    assert.ok(
      importIdx !== -1,
      `External @import must be present; got: ${nativeCss}`,
    );
    assert.ok(
      importIdx < localRuleIdx,
      `External @import must be hoisted before local rules; got: ${nativeCss}`,
    );

    // The external URL must be preserved verbatim.
    assert.ok(
      nativeCss.includes('fonts.googleapis.com'),
      `Hoisted @import must contain the external URL; got: ${nativeCss}`,
    );

    // The media query condition must be preserved alongside the hoisted @import.
    assert.ok(
      nativeCss.includes('screen') && nativeCss.includes('print'),
      `Hoisted @import must preserve the 'screen, print' media query; got: ${nativeCss}`,
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
      const mapContent = await overlayFS
        .readFile(mapPath, 'utf8')
        .catch(() => null);
      assert.ok(
        mapContent !== null,
        `Source map file must exist at ${mapPath} (fullNative=${featureFlags.fullNative})`,
      );

      // The .map file must be valid JSON with a 'sources' field.
      let mapJson: any;
      assert.doesNotThrow(() => {
        mapJson = JSON.parse(mapContent!);
      }, `Source map at ${mapPath} must be valid JSON (fullNative=${featureFlags.fullNative})`);
      assert.ok(
        Array.isArray(mapJson.sources) && mapJson.sources.length > 0,
        `Source map must have a non-empty 'sources' array (fullNative=${featureFlags.fullNative}); got: ${mapContent}`,
      );
    }
  });

  it('CSS Modules: renamed classes are present in output from both packagers', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-css-modules';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.js:
          import * as styles from './styles.module.css';
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

      // Both packagers must retain the used .title class (identified by its declaration).
      // CSS Modules renames the class, so we assert by the property value rather than
      // the selector name.
      assert.ok(
        cssContent.includes('font-weight'),
        `CSS output must contain 'font-weight' from used .title class (fullNative=${featureFlags.fullNative}); got: ${cssContent}`,
      );
    }

    // NOTE: CSS module tree-shaking (removing .unused) is a native-packager-specific
    // feature that requires end-to-end symbol propagation from the JS transformer.
    // It is verified in the css-modules.ts integration test suite via the
    // 'postcss-modules-import-namespace' fixture which has the necessary setup.
  });

  // NOTE: url() reference resolution requires an asset file type that has a
  // registered transformer (e.g. image types, SVG) or uses the raw packager.
  // Testing this end-to-end via fsFixture is complex because fsFixture disallows
  // binary image types and SVG goes through the SVG module transformer rather
  // than being treated as a raw URL asset. This behaviour is covered by the
  // url_replacer unit tests in crates/atlaspack_packager_css/src/url_replacer.rs.

  it('duplicate @import is included only once in output', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-dedup-import';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "./shared.css";
          @import "./other.css";
          .page { color: blue; }
        shared.css:
          .shared { font-size: 1rem; }
        other.css:
          @import "./shared.css";
          .other { color: green; }
        yarn.lock:
    `;

    const entryPath = path.join(__dirname, fixtureName, 'index.css');
    const bg = await bundle(entryPath, {
      mode: 'production' as const,
      inputFS: overlayFS,
      outputFS: overlayFS,
      featureFlags: {fullNative: true},
    });

    const nativeCss = await extractCssBundleContent(bg);

    // shared.css is imported by both index.css and other.css — it must appear
    // exactly once in the final output (deduplication).
    const matchCount = (nativeCss.match(/font-size/g) ?? []).length;
    assert.strictEqual(
      matchCount,
      1,
      `'font-size' from shared.css must appear exactly once (deduplication); got: ${nativeCss}`,
    );
  });
});
