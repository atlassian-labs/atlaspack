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

/// Reads the CSS bundles from a bundle graph and returns their content.
function extractCssBundleContents(bg: any): Promise<string[]> {
  const cssBundles = bg
    .getBundles()
    .filter((b: any) => b.type === 'css' && b.filePath)
    .sort((a: any, b: any) => a.filePath.localeCompare(b.filePath));
  assert.ok(cssBundles.length > 0, 'Expected at least one CSS bundle');
  return Promise.all(
    cssBundles.map((b: any) => overlayFS.readFile(b.filePath, 'utf8')),
  );
}

/// Builds `entry` via both JS and native packagers and returns each CSS output.
async function compareCssPackagers(
  fixtureName: string,
  mode: 'development' | 'production' = 'development',
  entries: string[] = ['index.css'],
): Promise<{jsContents: string[]; nativeContents: string[]}> {
  const entryPaths = entries.map((e) => path.join(__dirname, fixtureName, e));
  const commonOpts = {mode, inputFS: overlayFS, outputFS: overlayFS};

  const jsBundleGraph = await bundle(entryPaths, {
    ...commonOpts,
    featureFlags: {fullNative: false},
  });
  const jsContents = await extractCssBundleContents(jsBundleGraph);

  const nativeBundleGraph = await bundle(entryPaths, {
    ...commonOpts,
    featureFlags: {fullNative: true},
  });
  const nativeContents = await extractCssBundleContents(nativeBundleGraph);

  return {jsContents, nativeContents};
}

/// Asserts JS and native produce identical CSS in both dev and production modes.
async function assertPackagerParity(
  fixtureName: string,
  entries: string[] = ['index.css'],
): Promise<void> {
  for (const mode of ['development', 'production'] as const) {
    const {jsContents, nativeContents} = await compareCssPackagers(
      fixtureName,
      mode,
      entries,
    );
    assert.strictEqual(
      nativeContents.length,
      jsContents.length,
      `Bundle count must match in ${mode} mode.`,
    );
    for (let i = 0; i < jsContents.length; i++) {
      assert.strictEqual(
        nativeContents[i],
        jsContents[i],
        `CSS bundle #${i} must be identical in ${mode} mode.`,
      );
    }
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
    const fixtureName = 'packager-parity-ext-import';
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
    const fixtureName = 'packager-parity-dedup';

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

  it('url() references are resolved to identical relative paths', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-url-ref';

    // Both a plain url() and a url() with a #fragment must be resolved
    // identically: the placeholder token injected by the CSS transformer must
    // be replaced with the same relative path to the image output bundle.
    //
    // NOTE: In production mode the JS pipeline runs SVGO on SVG bundles,
    // changing both their content and content hash. The native packager emits
    // SVG bundles via a raw passthrough (no SVGO), so production hashes differ.
    // Until native SVG optimization is implemented, this test only asserts
    // parity in development mode.
    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .bg   { background-image: url('./hero.svg'); }
          .icon { background-image: url('./sprite.svg#arrow'); }
          .mask { mask-image: url('./hero.svg'); }
        hero.svg:
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="5" fill="blue"/></svg>
        sprite.svg:
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><polygon id="arrow" points="5,0 10,10 0,10" fill="red"/></svg>
        yarn.lock:
    `;

    // Development mode only — see note above.
    const {jsContents, nativeContents} = await compareCssPackagers(
      fixtureName,
      'development',
    );
    assert.strictEqual(nativeContents.length, jsContents.length);
    for (let i = 0; i < jsContents.length; i++) {
      assert.strictEqual(
        nativeContents[i],
        jsContents[i],
        `CSS bundle #${i} must be identical in development mode.`,
      );
    }
  });

  it('url() inline data URI is identical between packagers', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-url-inline';

    // data-url: scheme causes the asset to be inlined as a percent-encoded data
    // URI rather than emitted as a separate file.  Both packagers must produce
    // the same data:image/svg+xml,… token.
    //
    // NOTE: In production mode the JS pipeline runs the SVGO optimizer on the
    // inline SVG bundle (via atlaspack/optimizer/svgo) before encoding it as a
    // data URI.  The native CSS packager reads the raw transformer output from
    // the DB and has no equivalent SVGO pass, so production output differs.
    // Until native SVG optimization for inline data-url bundles is implemented,
    // this test only asserts parity in development mode.
    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .icon { background-image: url('data-url:./icon.svg'); }
        icon.svg:
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8" fill="green"/></svg>
        yarn.lock:
    `;

    // Development mode only — see note above.
    const {jsContents, nativeContents} = await compareCssPackagers(
      fixtureName,
      'development',
    );
    assert.strictEqual(nativeContents.length, jsContents.length);
    for (let i = 0; i < jsContents.length; i++) {
      assert.strictEqual(
        nativeContents[i],
        jsContents[i],
        `CSS bundle #${i} must be identical in development mode.`,
      );
    }
  });

  it('@font-face with local url() references is packaged identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-font-face';

    // Font files have no dedicated transformer and are passed through raw,
    // so dummy content is fine — the test exercises that @font-face is
    // preserved and that url() placeholders inside it are replaced correctly.
    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @font-face {
            font-family: 'Inter';
            src: url('./inter.woff2') format('woff2'),
                 url('./inter.woff') format('woff');
            font-weight: 400 700;
            font-display: swap;
          }
          .text { font-family: 'Inter', sans-serif; }
        inter.woff2:
          dummy
        inter.woff:
          dummy
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@layer cascade layers are preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-layer';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @layer reset, base, theme;
          @layer reset { *, *::before, *::after { box-sizing: border-box; } }
          @layer base   { body { margin: 0; font-family: system-ui; } }
          @layer theme  { :root { --color: #0052cc; } .btn { color: var(--color); } }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@supports feature queries are preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-supports';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @supports (display: grid) {
            .layout { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; }
          }
          @supports not (display: grid) {
            .layout { display: flex; flex-wrap: wrap; }
          }
          @supports (container-type: inline-size) {
            .card { container-type: inline-size; }
          }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@container queries are preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-container';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .sidebar { container-type: inline-size; container-name: sidebar; }
          @container sidebar (min-width: 300px) {
            .widget { display: flex; gap: 0.5rem; }
          }
          @container (min-width: 600px) {
            .card { grid-template-columns: 1fr 1fr; }
          }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('CSS nesting is preserved identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-nesting';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .button {
            padding: 0.5rem 1rem;
            & .icon { width: 1em; }
            &:hover { opacity: 0.85; }
            &:is(:focus, :focus-visible) { outline: 2px solid currentColor; }
            @media (max-width: 600px) { & { padding: 0.25rem 0.5rem; } }
          }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('CSS imported from a JS entry is packaged identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-js-entry';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.js:
          import './styles.css';
          export const version = 1;
        styles.css:
          @import './tokens.css';
          .container { display: flex; gap: var(--space); }
          .button { padding: 0.5rem 1rem; border-radius: 4px; }
        tokens.css:
          :root { --space: 1rem; --radius: 4px; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName, ['index.js']);
  });

  it('multiple CSS bundles from separate JS entries are packaged identically', async function () {
    this.timeout(30000);
    const fixtureName = 'packager-parity-multi-bundle';

    // Two independent JS entry points each pull in their own CSS tree.
    // Both packagers must produce the same set of CSS bundles (matched by
    // sort order) in both dev and production modes.
    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        entry-a.js:
          import './a.css';
          export const a = 1;
        entry-b.js:
          import './b.css';
          export const b = 2;
        a.css:
          @import './shared.css';
          .a-widget { background: #eef; }
        b.css:
          @import './shared.css';
          .b-widget { background: #efe; }
        shared.css:
          .base { font-family: system-ui; margin: 0; }
        yarn.lock:
    `;

    const entries = ['entry-a.js', 'entry-b.js'];

    await assertPackagerParity(fixtureName, entries);
  });
});
