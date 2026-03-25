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

/// Reads the first CSS bundle from a bundle graph and returns its content.
function extractCssBundleContent(bg: any): Promise<string> {
  const cssBundles = bg
    .getBundles()
    .filter((b: any) => b.type === 'css' && b.filePath);
  assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
  const filePath: string = cssBundles[0].filePath;
  return overlayFS.readFile(filePath, 'utf8');
}

/// Packages a CSS entry via both JS and native implementations and returns the output of each.
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

  // Read each output immediately, before the other packager overwrites in overlayFS.
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

/// Asserts that JS and native packagers produce identical output for `fixtureName`, in both development and production modes.
async function assertPackagerParity(fixtureName: string): Promise<void> {
  for (const mode of ['development', 'production'] as const) {
    const {jsCss, nativeCss} = await compareCssPackagers(fixtureName, mode);
    assert.strictEqual(
      nativeCss,
      jsCss,
      `Native and JS packagers must produce identical CSS in ${mode} mode.`,
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

    await assertPackagerParity(fixtureName);
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

    await assertPackagerParity(fixtureName);
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
});
