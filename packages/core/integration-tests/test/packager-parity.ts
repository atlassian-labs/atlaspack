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
 * Reads the first CSS bundle from a bundle graph and returns its normalised content.
 *
 * Both JS and native pipelines write via the FileSystemV3 bridge to overlayFS.
 * The native pipeline may use a content-hashed filename that differs from the
 * template path reported in the bundle graph — a dist-dir scan is used as fallback.
 */
async function extractCssBundleContent(bg: any): Promise<string> {
  const cssBundles = bg
    .getBundles()
    .filter((b: any) => b.type === 'css' && b.filePath);
  assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
  const filePath: string = cssBundles[0].filePath;

  // Both JS and native pipelines write to overlayFS via the FileSystemV3 bridge.
  // Each run uses its own distDir (set in compareCssPackagers) so their output
  // files never collide; filePath in the bundle graph is always resolvable.
  let css: string;
  try {
    css = await overlayFS.readFile(filePath, 'utf8');
  } catch {
    // Fallback: scan the dist dir in overlayFS for any .css file (handles cases
    // where the bundle graph reports a template path rather than the hashed name).
    const dir = path.dirname(filePath);
    const entries = await overlayFS.readdir(dir);
    const match = entries.find(
      (e: string) => e.endsWith('.css') && !e.endsWith('.map'),
    );
    assert.ok(
      match,
      `No CSS file found in ${dir}. Files: ${entries.join(', ')}`,
    );
    css = await overlayFS.readFile(path.join(dir, match!), 'utf8');
  }

  return css;
}

/**
 * Packages a CSS entry with both JS and native packagers and returns the
 * normalised CSS output for each.
 */
async function compareCssPackagers(
  fixtureName: string,
  mode: 'development' | 'production' = 'development',
): Promise<{
  jsCss: string;
  nativeCss: string;
}> {
  const entryPath = path.join(__dirname, fixtureName, 'index.css');
  const commonOpts = {
    mode,
    inputFS: overlayFS,
    outputFS: overlayFS,
  };

  // Read each packager's output immediately after its run, before the other
  // packager overwrites the same output path in overlayFS.
  const jsBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {fullNative: false},
  });
  const jsCss = await extractCssBundleContent(jsBundleGraph);

  const nativeBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {fullNative: true},
  });
  const nativeCss = await extractCssBundleContent(nativeBundleGraph);

  return {jsCss, nativeCss};
}

/**
 * Asserts that JS and native packagers produce identical normalised output for
 * `fixtureName`, in both development and production modes.
 */
async function assertPackagerParity(fixtureName: string): Promise<void> {
  for (const mode of ['development', 'production'] as const) {
    const {jsCss, nativeCss} = await compareCssPackagers(fixtureName, mode);
    assert.strictEqual(
      nativeCss,
      jsCss,
      `Native and JS packagers must produce identical normalised CSS in ${mode} mode.`,
    );
  }
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

    await assertPackagerParity(fixtureName);
  });

  it('multi-asset CSS concatenation produces identical output', async function () {
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

    await assertPackagerParity(fixtureName);
  });

  it('CSS custom properties round-trip identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-custom-props';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          :root { --color-brand: #0052cc; --spacing-md: 1rem; }
          .btn { color: var(--color-brand); padding: var(--spacing-md); }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@media query rules are preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-media-query';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .card { padding: 1rem; }
          @media (max-width: 768px) { .card { padding: 0.5rem; } }
          @media print { .card { display: none; } }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@keyframes animation rules are preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-keyframes';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
          .animated { animation: fade-in 0.3s ease-in-out; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('asset concatenation order matches source order', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-order';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "./first.css";
          @import "./second.css";
          .third { color: green; }
        first.css:
          .first { color: red; }
        second.css:
          .second { color: blue; }
        yarn.lock:
    `;

    for (const mode of ['development', 'production'] as const) {
      const {jsCss, nativeCss} = await compareCssPackagers(fixtureName, mode);

      // Verify parity.
      assert.strictEqual(
        nativeCss,
        jsCss,
        `Packagers must produce identical output in ${mode} mode.`,
      );

      // Independently verify the order is correct (first before second before third).
      const firstIdx = nativeCss.indexOf('.first');
      const secondIdx = nativeCss.indexOf('.second');
      const thirdIdx = nativeCss.indexOf('.third');
      assert.ok(
        firstIdx < secondIdx && secondIdx < thirdIdx,
        `Rules must appear in import order: first < second < third (${mode}); got: ${nativeCss}`,
      );
    }
  });

  it('external @import hoisting is identical between packagers', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-ext-import-parity';
    const extUrl = 'https://fonts.googleapis.com/css2?family=Inter';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "${extUrl}";
          .local { color: green; }
        yarn.lock:
    `;

    for (const mode of ['development', 'production'] as const) {
      const {jsCss, nativeCss} = await compareCssPackagers(fixtureName, mode);
      assert.strictEqual(
        nativeCss,
        jsCss,
        `External @import hoisting must be identical in ${mode} mode.`,
      );
    }
  });

  it('duplicate @import deduplication is identical between packagers', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-dedup-parity';

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

    await assertPackagerParity(fixtureName);
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

    const nativeCss = await extractCssBundleContent(nativeBundleGraph);

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

    // Both packagers must reference the same original source file(s) in their maps.
    const getCssBundlePath = (bg: any): string =>
      bg.getBundles().find((b: any) => b.type === 'css' && b.filePath).filePath;

    const jsBg = await bundle(path.join(__dirname, fixtureName, 'index.css'), {
      mode: 'development',
      inputFS: overlayFS,
      outputFS: overlayFS,
      featureFlags: {fullNative: false},
    });
    const nativeBg = await bundle(
      path.join(__dirname, fixtureName, 'index.css'),
      {
        mode: 'development',
        inputFS: overlayFS,
        outputFS: overlayFS,
        featureFlags: {fullNative: true},
      },
    );

    const jsMap = JSON.parse(
      await overlayFS.readFile(getCssBundlePath(jsBg) + '.map', 'utf8'),
    );
    const nativeMap = JSON.parse(
      await overlayFS.readFile(getCssBundlePath(nativeBg) + '.map', 'utf8'),
    );

    // Normalise source paths (strip any file:// prefix or absolute path prefix) to
    // compare only the basename(s), since the two packagers may use different path formats.
    const basenames = (sources: string[]) =>
      sources.map((s) => path.basename(s)).sort();

    assert.deepStrictEqual(
      basenames(nativeMap.sources),
      basenames(jsMap.sources),
      `Both packagers must reference the same source files in their source maps.\nNative: ${JSON.stringify(nativeMap.sources)}\nJS:     ${JSON.stringify(jsMap.sources)}`,
    );
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

  it('duplicate @import appears exactly once in native output', async function () {
    this.timeout(30000);
    // This test verifies the native packager's deduplication independently.
    // Cross-packager parity of deduplication is verified by the
    // 'duplicate @import deduplication is identical between packagers' test above.
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
