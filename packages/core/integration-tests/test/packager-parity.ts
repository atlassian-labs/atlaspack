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

const BuildMode = {
  DEVELOPMENT: 'development',
  PRODUCTION: 'production',
} as const;
type BuildMode = (typeof BuildMode)[keyof typeof BuildMode];

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
  buildMode: BuildMode,
  entries: string[],
): Promise<{jsContents: string[]; nativeContents: string[]}> {
  const entryPaths = entries.map((e) => path.join(__dirname, fixtureName, e));
  const commonOpts = {
    mode: buildMode,
    inputFS: overlayFS,
    outputFS: overlayFS,
  };

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
  {
    entries = ['index.css'],
    buildModes = [BuildMode.DEVELOPMENT, BuildMode.PRODUCTION],
  } = {},
): Promise<void> {
  for (const mode of buildModes) {
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

  beforeEach(function () {
    this.timeout(30000);
  });

  it('simple single-file CSS is packaged identically', async function () {
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

    await assertPackagerParity(fixtureName, {
      buildModes: [BuildMode.DEVELOPMENT],
    });
  });

  it('url() inline data URI is identical between packagers', async function () {
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

    await assertPackagerParity(fixtureName, {
      buildModes: [BuildMode.DEVELOPMENT],
    });
  });

  it('@font-face with local url() references is packaged identically', async function () {
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

    await assertPackagerParity(fixtureName, {entries: ['index.js']});
  });

  it('multiple CSS bundles from separate JS entries are packaged identically', async function () {
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

    await assertPackagerParity(fixtureName, {
      entries: ['entry-a.js', 'entry-b.js'],
    });
  });

  it('CSS Modules wildcard import retains all classes identically in development mode', async function () {
    // In production mode the JS packager activates its processCSSModule (PostCSS) path
    // while the native packager uses lightningcss, producing different whitespace.
    // Development mode passes the raw transformer output through both packagers unchanged,
    // so strict equality holds.
    const fixtureName = 'packager-parity-css-modules-wildcard';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.js:
          import * as styles from './styles.module.css';
          document.body.className = styles.foo;
        styles.module.css:
          .foo { color: red; }
          .bar { color: blue; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName, {
      entries: ['index.js'],
      buildModes: [BuildMode.DEVELOPMENT],
    });
  });

  it('@import media query conditions are stripped identically by both packagers', async function () {
    // Both packagers reconstruct hoisted external @imports using only the URL —
    // media query conditions (e.g. `screen, print`) are not preserved by either packager.
    // This test guards against one packager accidentally preserving conditions
    // while the other strips them, causing a divergence.
    const fixtureName = 'packager-parity-ext-import-mq-conditions';
    const extUrl = 'https://fonts.googleapis.com/css2?family=Lato';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "${extUrl}" screen, print;
          .local { color: green; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@starting-style rules are preserved identically', async function () {
    const fixtureName = 'packager-parity-starting-style';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          .dialog {
            transition: opacity 0.3s, display 0.3s allow-discrete;
            opacity: 1;
          }
          @starting-style {
            .dialog {
              opacity: 0;
            }
          }
          .popover {
            transition: transform 0.2s;
            transform: scale(1);
          }
          @starting-style {
            .popover {
              transform: scale(0.8);
            }
          }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('@scope rules are preserved identically', async function () {
    const fixtureName = 'packager-parity-scope';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @scope (.card) {
            .title { font-size: 1.25rem; font-weight: bold; }
            .body  { padding: 1rem; }
          }
          @scope (.sidebar) to (.widget) {
            p { margin: 0; color: var(--sidebar-text); }
          }
          :root { --sidebar-text: #333; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('compiled atomic CSS (Atlassian Compiled-style output) is packaged identically', async function () {
    // Mimics the atomic class-per-property CSS emitted by @compiled/react.
    // Each rule has a single declaration with a hashed class name and an
    // Atlassian Design System token as the value, mimicking many real
    // compiled.*.css files. The volume (~100 rules) is representative of a
    // single page-level bundle.
    const fixtureName = 'packager-parity-compiled-atomic';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        tokens.css:
          :root {
            --ds-surface: #fff;
            --ds-text: #172b4d;
            --ds-text-subtle: #505258;
            --ds-text-inverse: #fff;
            --ds-link: #0052cc;
            --ds-link-pressed: #0747a6;
            --ds-background-neutral: #f4f5f7;
            --ds-background-neutral-hovered: #ebecf0;
            --ds-background-neutral-pressed: #dfe1e6;
            --ds-background-selected: #deebff;
            --ds-background-selected-hovered: #b3d4ff;
            --ds-background-selected-pressed: #4c9aff;
            --ds-background-danger: #ffebe6;
            --ds-background-danger-hovered: #ffbdad;
            --ds-background-success: #e3fcef;
            --ds-background-success-hovered: #abf5d1;
            --ds-background-warning: #fffae6;
            --ds-background-warning-hovered: #ffe380;
            --ds-border: #dfe1e6;
            --ds-border-focused: #2684ff;
            --ds-border-danger: #de350b;
            --ds-border-success: #00875a;
            --ds-border-width-focused: 2px;
            --ds-space-025: 2px;
            --ds-space-050: 4px;
            --ds-space-075: 6px;
            --ds-space-100: 8px;
            --ds-space-150: 12px;
            --ds-space-200: 16px;
            --ds-space-250: 20px;
            --ds-space-300: 24px;
            --ds-space-400: 32px;
            --ds-space-500: 40px;
            --ds-space-600: 48px;
            --ds-font-family-body: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, "Helvetica Neue", sans-serif;
            --ds-font-family-heading: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, "Helvetica Neue", sans-serif;
            --ds-font-family-monospace: ui-monospace, "Menlo", "Monaco", "Cascadia Mono", "Segoe UI Mono", monospace;
            --ds-font-body: normal 400 14px/1.42857 var(--ds-font-family-body);
            --ds-font-heading-xxlarge: 600 2.57143em/1.11111 var(--ds-font-family-heading);
            --ds-font-heading-xlarge: 600 2.07143em/1.10345 var(--ds-font-family-heading);
            --ds-font-heading-large: 500 1.71429em/1.16667 var(--ds-font-family-heading);
            --ds-font-heading-medium: 500 1.42857em/1.2 var(--ds-font-family-heading);
            --ds-font-heading-small: 600 1.14286em/1.25 var(--ds-font-family-heading);
            --ds-font-heading-xsmall: 600 1em/1.14286 var(--ds-font-family-heading);
            --ds-font-heading-xxsmall: 600 0.85714em/1.33333 var(--ds-font-family-heading);
            --ds-shadow-raised: 0px 1px 1px #091e4240, 0px 0px 1px #091e424f;
            --ds-shadow-overlay: 0px 8px 12px #091e4226, 0px 0px 1px #091e424f;
            --ds-opacity-loading: 0.2;
            --ds-opacity-disabled: 0.4;
          }
        compiled.css:
          ._19itidpf{border:0}
          ._19itglyw{border:none}
          ._ect41gqc{font-family:var(--ds-font-family-body,ui-sans-serif,-apple-system,BlinkMacSystemFont,"Segoe UI",Ubuntu,"Helvetica Neue",sans-serif)}
          ._4bfu1r31{text-decoration-color:currentColor}
          ._1hms8stv{text-decoration-line:underline}
          ._ajmmnqa1{text-decoration-style:solid}
          ._syaz13af{color:var(--ds-link,#1868db)}
          ._1hmsglyw{text-decoration-line:none}
          ._syazazsu{color:var(--ds-text-subtle,#505258)}
          ._syaz15cr{color:var(--ds-text-inverse,#fff)}
          ._1e0c1nu9{display:inline}
          ._o5721q9c{white-space:nowrap}
          ._s7n41q9y{vertical-align:baseline}
          ._kqswh2mm{position:relative}
          ._152ttb3r{inset-block-start:.11em}
          ._1bsbgm0b{width:.9em}
          ._4t3igm0b{height:.9em}
          ._ahbqzjw7{margin-inline-start:.3em}
          ._vchhusvi{box-sizing:border-box}
          ._ca0qidpf{padding-top:0}
          ._u5f3idpf{padding-right:0}
          ._n3tdidpf{padding-bottom:0}
          ._19bvidpf{padding-left:0}
          ._1reo15vq{overflow-x:hidden}
          ._18m915vq{overflow-y:hidden}
          ._1bsbt94y{width:1px}
          ._4t3it94y{height:1px}
          ._kqswstnw{position:absolute}
          ._ogto7mnp{clip:rect(1px,1px,1px,1px)}
          ._uiztglyw{-webkit-user-select:none;user-select:none}
          ._ymio1r31:focus:not(:focus-visible){outline-color:currentColor}
          ._ypr0glyw:focus:not(:focus-visible){outline-style:none}
          ._zcxs1o36:focus:not(:focus-visible){outline-width:medium}
          ._r06hglyw{-webkit-appearance:none;-moz-appearance:none;appearance:none}
          ._bfhkm890{background-color:var(--ds-background-neutral,#f4f5f7)}
          ._bfhkd4y8{background-color:var(--ds-background-neutral-hovered,#ebecf0)}
          ._bfhkkuup{background-color:var(--ds-background-neutral-pressed,#dfe1e6)}
          ._bfhkz2ec{background-color:var(--ds-background-selected,#deebff)}
          ._bfhk1gf0{background-color:var(--ds-background-selected-hovered,#b3d4ff)}
          ._bfhk2kxc{background-color:var(--ds-background-selected-pressed,#4c9aff)}
          ._bfhkbq5w{background-color:var(--ds-background-danger,#ffebe6)}
          ._bfhkfoww{background-color:var(--ds-background-danger-hovered,#ffbdad)}
          ._bfhk1jbd{background-color:var(--ds-background-success,#e3fcef)}
          ._bfhkabc1{background-color:var(--ds-background-success-hovered,#abf5d1)}
          ._bfhkabc2{background-color:var(--ds-background-warning,#fffae6)}
          ._bfhkabc3{background-color:var(--ds-background-warning-hovered,#ffe380)}
          ._syaz1234{color:var(--ds-text,#172b4d)}
          ._syaz5678{color:var(--ds-text-subtle,#626f86)}
          ._1wybidpf{flex-grow:0}
          ._1wyb1234{flex-grow:1}
          ._80omidpf{flex-shrink:0}
          ._80om1234{flex-shrink:1}
          ._1wybidpf{flex-grow:0}
          ._4cvr1234{flex-direction:row}
          ._4cvr5678{flex-direction:column}
          ._4cvr9abc{flex-direction:row-reverse}
          ._1qag1234{align-items:center}
          ._1qag5678{align-items:flex-start}
          ._1qag9abc{align-items:flex-end}
          ._1qagdef0{align-items:stretch}
          ._vcv11234{justify-content:center}
          ._vcv15678{justify-content:flex-start}
          ._vcv19abc{justify-content:flex-end}
          ._vcv1def0{justify-content:space-between}
          ._11c8idpf{gap:0}
          ._11c81234{gap:var(--ds-space-050,4px)}
          ._11c85678{gap:var(--ds-space-100,8px)}
          ._11c89abc{gap:var(--ds-space-200,16px)}
          ._11c8def0{gap:var(--ds-space-300,24px)}
          ._otyridpf{padding:0}
          ._otyr1234{padding:var(--ds-space-050,4px)}
          ._otyr5678{padding:var(--ds-space-100,8px)}
          ._otyr9abc{padding:var(--ds-space-200,16px)}
          ._1234abcd{margin:0 auto}
          ._5678abcd{width:100%}
          ._9abcdef0{max-width:1280px}
          ._def01234{height:100vh}
          ._abcd1234{min-height:0}
          ._1234efgh{overflow:hidden}
          ._5678efgh{overflow:auto}
          ._9abcefgh{overflow:visible}
          ._border12{border:var(--ds-border-width-focused,2px) solid var(--ds-border,#dfe1e6)}
          ._border34{border-radius:var(--ds-space-050,4px)}
          ._border56{border-radius:var(--ds-space-075,6px)}
          ._border78{border-color:var(--ds-border-focused,#2684ff)}
          ._border9a{border-color:var(--ds-border-danger,#de350b)}
          ._borderbc{border-color:var(--ds-border-success,#00875a)}
          ._opac1234{opacity:var(--ds-opacity-disabled,0.4)}
          ._opac5678{opacity:var(--ds-opacity-loading,0.2)}
          ._opac9abc{opacity:1}
          ._trans1234{transition:background-color 0.2s ease,color 0.2s ease,box-shadow 0.2s ease}
          ._trans5678{transition:opacity 0.2s ease}
          ._trans9abc{transition:transform 0.15s ease-out}
          ._curs1234{cursor:pointer}
          ._curs5678{cursor:not-allowed}
          ._curs9abc{cursor:default}
          ._outl1234{outline:none}
          ._outl5678{outline:var(--ds-border-width-focused,2px) solid var(--ds-border-focused,#2684ff)}
          ._outl9abc{outline-offset:var(--ds-space-025,2px)}
          ._zidx1234{z-index:100}
          ._zidx5678{z-index:200}
          ._zidx9abc{z-index:300}
          ._zidxdef0{z-index:400}
        index.css:
          @import "./tokens.css";
          @import "./compiled.css";
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('design system global reset with tokens is packaged identically', async function () {
    const fixtureName = 'packager-parity-ds-reset';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        reset.css:
          html,body,p,div,h1,h2,h3,h4,h5,h6,ul,ol,dl,img,pre,form,fieldset { margin: 0; padding: 0; }
          img,fieldset { border: 0; }
          body, html { width: 100%; height: 100%; }
          body {
            background-color: var(--ds-surface, #fff);
            color: var(--ds-text, #172b4d);
            font: var(--ds-font-body, normal 400 14px/1.42857 -apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Fira Sans", "Droid Sans", "Helvetica Neue", sans-serif);
            -ms-overflow-style: -ms-autohiding-scrollbar;
            -webkit-text-decoration-skip-ink: auto;
            text-decoration-skip-ink: auto;
          }
          p,ul,ol,dl,h1,h2,h3,h4,h5,h6,blockquote,pre,form,table { margin: var(--ds-space-150, 12px) 0 0 0; }
          a { color: var(--ds-link, #0052cc); text-decoration: none; }
          a:hover { color: var(--ds-link, #0065ff); text-decoration: underline; }
          a:active { color: var(--ds-link-pressed, #0747a6); }
          a:focus-visible {
            outline: var(--ds-border-width-focused, 2px) solid var(--ds-border-focused, #2684ff);
            outline-offset: var(--ds-space-025, 2px);
          }
          @supports not selector(*:focus-visible) {
            a:focus {
              outline: var(--ds-border-width-focused, 2px) solid var(--ds-border-focused, #4c9aff);
              outline-offset: var(--ds-space-025, 2px);
            }
          }
          h1 { font: var(--ds-font-heading-xlarge, 600 2.07143em/1.10345 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: var(--ds-space-500, 40px); }
          h2 { font: var(--ds-font-heading-large, 500 1.71429em/1.16667 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: var(--ds-space-500, 40px); }
          h3 { font: var(--ds-font-heading-medium, 500 1.42857em/1.2 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: 28px; }
          h4 { font: var(--ds-font-heading-small, 600 1.14286em/1.25 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: var(--ds-space-300, 24px); }
          h5 { font: var(--ds-font-heading-xsmall, 600 1em/1.14286 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: var(--ds-space-200, 16px); }
          h6 { font: var(--ds-font-heading-xxsmall, 600 0.85714em/1.33333 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif); color: var(--ds-text); margin-top: var(--ds-space-150, 12px); }
          code, kbd, pre { font-family: var(--ds-font-family-monospace, ui-monospace, "Menlo", "Monaco", "Cascadia Mono", monospace); }
          blockquote {
            border-left: 2px solid var(--ds-border, #dfe1e6);
            color: var(--ds-text-subtle, #626f86);
            padding-left: var(--ds-space-200, 16px);
            margin: var(--ds-space-200, 16px) 0;
          }
          table { border-collapse: collapse; width: 100%; }
          th, td { padding: var(--ds-space-100, 8px) var(--ds-space-150, 12px); border: 1px solid var(--ds-border, #dfe1e6); text-align: left; }
          th { background-color: var(--ds-background-neutral, #f4f5f7); font-weight: 600; }
          input, button, select, textarea { font-family: inherit; font-size: inherit; }
          button { cursor: pointer; }
          :focus-visible { outline: var(--ds-border-width-focused, 2px) solid var(--ds-border-focused, #2684ff); outline-offset: 2px; }
          *,*::before,*::after { box-sizing: border-box; }
        typography.css:
          .heading-xxlarge {
            font: var(--ds-font-heading-xxlarge, 600 2.57143em/1.11111 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif);
            color: var(--ds-text);
          }
          .heading-xlarge { font: var(--ds-font-heading-xlarge, 600 2.07143em/1.10345 -apple-system, sans-serif); color: var(--ds-text); }
          .heading-large  { font: var(--ds-font-heading-large, 500 1.71429em/1.16667 -apple-system, sans-serif); color: var(--ds-text); }
          .heading-medium { font: var(--ds-font-heading-medium, 500 1.42857em/1.2 -apple-system, sans-serif); color: var(--ds-text); }
          .body { font: var(--ds-font-body, normal 400 14px/1.42857 -apple-system, sans-serif); color: var(--ds-text); }
          .body-small { font-size: 0.85714em; line-height: 1.33333; color: var(--ds-text-subtle, #626f86); }
          .body-bold { font-weight: 700; }
          .truncate { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; max-width: 100%; }
          .sr-only {
            position: absolute; width: 1px; height: 1px;
            padding: 0; margin: -1px; overflow: hidden;
            clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
          }
        index.css:
          @import "./reset.css";
          @import "./typography.css";
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('multi-file component bundle with DS tokens and media queries is packaged identically', async function () {
    // Mimics a realistic component-tree CSS bundle from a product pages:
    // a shared token layer, a layout system, several component files (nav, sidebar,
    // content, cards), responsive breakpoints, and focus/hover/active states.
    // This exercises the packager under volume and import-chain depth simultaneously.
    const fixtureName = 'packager-parity-component-bundle';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        tokens.css:
          :root {
            --ds-surface: #fff;
            --ds-surface-sunken: #f4f5f7;
            --ds-surface-overlay: #fff;
            --ds-text: #172b4d;
            --ds-text-subtle: #626f86;
            --ds-text-disabled: #8993a4;
            --ds-link: #0052cc;
            --ds-background-neutral: #f4f5f7;
            --ds-background-selected: #deebff;
            --ds-border: #dfe1e6;
            --ds-border-focused: #2684ff;
            --ds-shadow-raised: 0px 1px 1px #091e4240, 0px 0px 1px #091e424f;
            --ds-shadow-overlay: 0px 8px 12px #091e4226, 0px 0px 1px #091e424f;
            --ds-space-050: 4px;
            --ds-space-100: 8px;
            --ds-space-150: 12px;
            --ds-space-200: 16px;
            --ds-space-300: 24px;
            --ds-space-400: 32px;
            --ds-space-500: 40px;
            --ds-space-600: 48px;
            --page-max-width: 1280px;
            --nav-height: 56px;
            --sidebar-width: 240px;
            --sidebar-collapsed-width: 48px;
          }
        layout.css:
          .page-root {
            display: flex;
            flex-direction: column;
            min-height: 100vh;
            background-color: var(--ds-surface-sunken, #f4f5f7);
          }
          .page-wrapper {
            display: flex;
            flex: 1 1 auto;
            max-width: var(--page-max-width, 1280px);
            margin: 0 auto;
            width: 100%;
            padding: 0 var(--ds-space-200, 16px);
          }
          .page-content {
            flex: 1 1 auto;
            min-width: 0;
            padding: var(--ds-space-300, 24px) var(--ds-space-400, 32px);
          }
          @media (max-width: 768px) {
            .page-wrapper { flex-direction: column; padding: 0; }
            .page-content { padding: var(--ds-space-200, 16px) var(--ds-space-150, 12px); }
          }
          @media (max-width: 480px) {
            .page-content { padding: var(--ds-space-150, 12px) var(--ds-space-100, 8px); }
          }
        nav.css:
          .nav-root {
            position: sticky;
            top: 0;
            z-index: 300;
            height: var(--nav-height, 56px);
            background-color: var(--ds-surface, #fff);
            border-bottom: 1px solid var(--ds-border, #dfe1e6);
            display: flex;
            align-items: center;
            padding: 0 var(--ds-space-300, 24px);
            box-shadow: var(--ds-shadow-raised, 0 1px 2px rgba(0,0,0,0.16));
          }
          .nav-logo { flex-shrink: 0; margin-right: var(--ds-space-200, 16px); }
          .nav-search {
            flex: 1 1 auto;
            max-width: 480px;
            margin: 0 var(--ds-space-300, 24px);
          }
          .nav-actions { display: flex; align-items: center; gap: var(--ds-space-100, 8px); margin-left: auto; }
          .nav-avatar {
            width: 32px; height: 32px;
            border-radius: 50%;
            cursor: pointer;
          }
          .nav-avatar:hover { box-shadow: 0 0 0 2px var(--ds-border-focused, #2684ff); }
          .nav-item {
            display: flex;
            align-items: center;
            gap: var(--ds-space-050, 4px);
            padding: var(--ds-space-075, 6px) var(--ds-space-100, 8px);
            border-radius: 3px;
            color: var(--ds-text, #172b4d);
            text-decoration: none;
            cursor: pointer;
            transition: background-color 0.1s ease;
          }
          .nav-item:hover { background-color: var(--ds-background-neutral, #f4f5f7); }
          .nav-item:active { background-color: var(--ds-background-neutral-pressed, #dfe1e6); }
          .nav-item[aria-current='page'] { background-color: var(--ds-background-selected, #deebff); color: var(--ds-link, #0052cc); }
          .nav-item:focus-visible { outline: 2px solid var(--ds-border-focused, #2684ff); outline-offset: 2px; }
          @media (max-width: 768px) {
            .nav-search { display: none; }
            .nav-root { padding: 0 var(--ds-space-150, 12px); }
          }
        sidebar.css:
          .sidebar {
            flex-shrink: 0;
            width: var(--sidebar-width, 240px);
            background-color: var(--ds-surface, #fff);
            border-right: 1px solid var(--ds-border, #dfe1e6);
            padding: var(--ds-space-200, 16px) 0;
            overflow-y: auto;
            transition: width 0.2s ease;
          }
          .sidebar--collapsed { width: var(--sidebar-collapsed-width, 48px); }
          .sidebar--collapsed .sidebar-label { opacity: 0; width: 0; overflow: hidden; }
          .sidebar-item {
            display: flex;
            align-items: center;
            gap: var(--ds-space-100, 8px);
            padding: var(--ds-space-075, 6px) var(--ds-space-200, 16px);
            color: var(--ds-text, #172b4d);
            text-decoration: none;
            cursor: pointer;
            border-radius: 3px;
            margin: 0 var(--ds-space-100, 8px);
          }
          .sidebar-item:hover { background-color: var(--ds-background-neutral, #f4f5f7); }
          .sidebar-item--active {
            background-color: var(--ds-background-selected, #deebff);
            color: var(--ds-link, #0052cc);
            font-weight: 500;
          }
          .sidebar-section-title {
            font-size: 11px;
            font-weight: 700;
            letter-spacing: 0.04em;
            text-transform: uppercase;
            color: var(--ds-text-subtle, #626f86);
            padding: var(--ds-space-100, 8px) var(--ds-space-200, 16px) var(--ds-space-050, 4px);
          }
          @media (max-width: 768px) {
            .sidebar { display: none; }
          }
        card.css:
          .card {
            background-color: var(--ds-surface, #fff);
            border: 1px solid var(--ds-border, #dfe1e6);
            border-radius: 8px;
            padding: var(--ds-space-300, 24px);
            box-shadow: var(--ds-shadow-raised, 0 1px 2px rgba(0,0,0,0.08));
            transition: box-shadow 0.15s ease;
          }
          .card:hover { box-shadow: var(--ds-shadow-overlay, 0 4px 8px rgba(0,0,0,0.16)); }
          .card--flat { box-shadow: none; }
          .card--flat:hover { box-shadow: none; background-color: var(--ds-surface-sunken, #f4f5f7); }
          .card-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: var(--ds-space-200, 16px);
          }
          .card-title {
            font-size: 16px;
            font-weight: 600;
            color: var(--ds-text, #172b4d);
            line-height: 1.25;
          }
          .card-body { color: var(--ds-text, #172b4d); font-size: 14px; line-height: 1.42857; }
          .card-footer {
            display: flex;
            align-items: center;
            gap: var(--ds-space-100, 8px);
            margin-top: var(--ds-space-200, 16px);
            padding-top: var(--ds-space-200, 16px);
            border-top: 1px solid var(--ds-border, #dfe1e6);
          }
          .card-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: var(--ds-space-200, 16px);
          }
          @media (max-width: 480px) {
            .card-grid { grid-template-columns: 1fr; }
            .card { padding: var(--ds-space-200, 16px); border-radius: 4px; }
          }
        index.css:
          @import "./tokens.css";
          @import "./nav.css";
          @import "./sidebar.css";
          @import "./layout.css";
          @import "./card.css";
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it('keyframes, animations, @supports, and media queries are packaged identically', async function () {
    // Exercises the at-rule coverage that matters most for animated product
    // UI: loading skeletons, spinners, toast/flag entry animations, focus rings,
    // and the @supports fallbacks Confluence uses for focus-visible polyfilling.
    const fixtureName = 'packager-parity-at-rules';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        animations.css:
          @keyframes spin {
            from { transform: rotate(0deg); }
            to   { transform: rotate(360deg); }
          }
          @keyframes pulse {
            0%, 100% { opacity: 1; }
            50%       { opacity: 0.4; }
          }
          @keyframes skeleton-shimmer {
            0%   { background-position: -200% 0; }
            100% { background-position:  200% 0; }
          }
          @keyframes slide-in-from-right {
            from { transform: translateX(100%); opacity: 0; }
            to   { transform: translateX(0);    opacity: 1; }
          }
          @keyframes slide-out-to-right {
            from { transform: translateX(0);    opacity: 1; }
            to   { transform: translateX(100%); opacity: 0; }
          }
          @keyframes fade-in {
            from { opacity: 0; }
            to   { opacity: 1; }
          }
          @keyframes scale-in {
            from { transform: scale(0.9); opacity: 0; }
            to   { transform: scale(1);   opacity: 1; }
          }
          @keyframes highlight-pulse {
            0%        { box-shadow: none; }
            10%, 90%  { box-shadow: 0 0 0 3px var(--ds-border-focused, #2684ff); }
            100%      { box-shadow: none; }
          }
          .spinner {
            animation: spin 0.7s linear infinite;
            border: 2px solid var(--ds-border, #dfe1e6);
            border-top-color: var(--ds-link, #0052cc);
            border-radius: 50%;
            width: 20px;
            height: 20px;
          }
          .skeleton {
            background: linear-gradient(
              90deg,
              var(--ds-background-neutral, #f4f5f7) 25%,
              var(--ds-background-neutral-hovered, #ebecf0) 50%,
              var(--ds-background-neutral, #f4f5f7) 75%
            );
            background-size: 400% 100%;
            animation: skeleton-shimmer 1.4s ease infinite;
            border-radius: 3px;
          }
          .skeleton--text  { height: 14px; margin-bottom: 8px; }
          .skeleton--title { height: 24px; width: 60%; margin-bottom: 16px; }
          .skeleton--avatar { width: 32px; height: 32px; border-radius: 50%; }
          .flag {
            animation: slide-in-from-right 0.3s cubic-bezier(0.2, 0, 0, 1) forwards;
          }
          .flag--exit {
            animation: slide-out-to-right 0.25s cubic-bezier(0.2, 0, 1, 1) forwards;
          }
          .toast-enter { animation: fade-in 0.15s ease forwards; }
          .popover-enter { animation: scale-in 0.15s cubic-bezier(0.2, 0, 0, 1) forwards; }
          .highlight { animation: highlight-pulse 2s ease 1 forwards; }
          @media (prefers-reduced-motion: reduce) {
            .spinner, .skeleton, .flag, .flag--exit, .toast-enter, .popover-enter, .highlight {
              animation: none;
            }
          }
        focus.css:
          @supports selector(*:focus-visible) {
            :focus:not(:focus-visible) { outline: none; }
            :focus-visible {
              outline: var(--ds-border-width-focused, 2px) solid var(--ds-border-focused, #2684ff);
              outline-offset: 2px;
            }
          }
          @supports not selector(*:focus-visible) {
            :focus {
              outline: var(--ds-border-width-focused, 2px) solid var(--ds-border-focused, #4c9aff);
              outline-offset: 2px;
            }
          }
          @media (prefers-color-scheme: dark) {
            :root {
              --ds-surface: #1d2125;
              --ds-surface-sunken: #161a1d;
              --ds-text: #b6c2cf;
              --ds-text-subtle: #738496;
              --ds-border: #2c333a;
              --ds-background-neutral: #22272b;
              --ds-link: #579dff;
              --ds-border-focused: #579dff;
            }
          }
          @media print {
            .no-print { display: none !important; }
            .print-only { display: block !important; }
            * { box-shadow: none !important; animation: none !important; }
          }
        index.css:
          @import "./animations.css";
          @import "./focus.css";
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });

  it.skip('diamond import deduplication is identical between packagers', async function () {
    // TODO: Native packager uses *correct* DFS post-order (shared.css before a-component).
    // JS packager orders incorrectly due to lightning walking through internal @imports.
    // Fix would be in CSSPackager.ts.
    const fixtureName = 'packager-parity-diamond-dedup';

    await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        index.css:
          @import "./a.css";
          @import "./b.css";
          .page { padding: 2rem; }
        a.css:
          @import "./shared.css";
          .a-component { color: blue; }
        b.css:
          @import "./shared.css";
          .b-component { color: green; }
        shared.css:
          *, *::before, *::after { box-sizing: border-box; }
          body { margin: 0; font-family: system-ui; }
        yarn.lock:
    `;

    await assertPackagerParity(fixtureName);
  });
});
