import assert from 'node:assert';
import {describe, it, before, after, beforeEach, afterEach} from 'node:test';
import {chromium} from 'playwright';
import type {Browser, Page, BrowserContext} from 'playwright';
import {buildFixture} from '../utils/build-fixture.mts';
import {serve} from '../utils/server.mts';
import type {ServeContext} from '../utils/server.mts';
import {join, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('Compiled CSS in JS Playwright E2E tests', () => {
  let server: ServeContext | undefined;
  let browser: Browser;
  let context: BrowserContext;
  let page: Page;

  before(async () => {
    browser = await chromium.launch();
  });

  after(async () => {
    await browser.close();
  });

  beforeEach(async () => {
    context = await browser.newContext();
    page = await context.newPage();
  });

  afterEach(async () => {
    await context.close();
    if (server) server.close();
  });

  it('can bundle a project with compiled CSS in JS natively', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-compiled-css-in-js-natively/index.html',
      {
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
        shouldDisableCache: true,
        featureFlags: {
          compiledCssInJsTransformer: true,
        },
        config: join(
          __dirname,
          'data/simple-project-with-compiled-css-in-js-natively/.atlaspackrc',
        ),
      },
    );

    server = await serve(outputDir);
    await page.goto(server.address);

    const headingElement = page.getByTestId('heading');
    assert.equal(await headingElement.innerText(), 'Hello, world!');
    const color = await headingElement.evaluate(
      (el) => getComputedStyle(el).color,
    );
    assert.equal(color, 'rgb(255, 0, 0)');

    const buttonElement = page.getByTestId('button');
    const cursor = await buttonElement.evaluate(
      (el) => getComputedStyle(el).cursor,
    );
    assert.equal(cursor, 'pointer');
  });

  it('can bundle a project with compiled CSS in JS natively with extraction', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-compiled-css-in-js-extracted/index.html',
      {
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
        shouldDisableCache: true,
        featureFlags: {
          compiledCssInJsTransformer: true,
        },
        config: join(
          __dirname,
          'data/simple-project-with-compiled-css-in-js-extracted/.atlaspackrc',
        ),
      },
    );

    server = await serve(outputDir);
    await page.goto(server.address);

    const headingElement = page.getByTestId('heading');
    assert.equal(await headingElement.innerText(), 'Hello, world!');
    const color = await headingElement.evaluate(
      (el) => getComputedStyle(el).color,
    );
    assert.equal(color, 'rgb(255, 0, 0)');

    const buttonElement = page.getByTestId('button');
    const cursor = await buttonElement.evaluate(
      (el) => getComputedStyle(el).cursor,
    );
    assert.equal(cursor, 'pointer');
  });

  /**
   * Shared assertion helper for cssMapScoped fixtures. Both the extracted and
   * runtime variants render the same JSX (three editor-panel <div>s styled via
   * the `css={[base, override1, override2]}` array form) and should result in
   * the same on-page behaviour regardless of how the CSS sheets are delivered.
   *
   * Covers:
   *   - nested descendant selectors (`.editor .panel`, `.editor .panel-title`)
   *   - `css={[a, b, c]}` array form with three cssMapScoped variants
   *   - overlapping variant keys producing UNIQUE class names
   *   - `@media` queries inside variants
   *   - `@keyframes` via `keyframes()` (global, not scoped)
   *   - parent-pseudo selectors (`&:hover`, `&:focus`) correctly flattened
   *   - autoprefixer-driven vendor prefixes (e.g. `-webkit-user-select`)
   */
  async function assertCssMapScopedFixture(fixtureName: string) {
    const {outputDir} = await buildFixture(`${fixtureName}/index.html`, {
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
      shouldDisableCache: true,
      featureFlags: {
        compiledCssInJsTransformer: true,
      },
      config: join(__dirname, `data/${fixtureName}/.atlaspackrc`),
    });

    server = await serve(outputDir);
    await page.goto(server.address);

    // Helper to grab the inner `.panel` of a test panel — the wrapper carries
    // the `.cc-<hash>` class, the inner `.panel` is what receives the scoped
    // declarations (`.cc-xxx .editor .panel { ... }`).
    const innerPanel = (testid: string) =>
      page.locator(`[data-testid="${testid}"] .panel`);
    const innerPanelTitle = (testid: string) =>
      page.locator(`[data-testid="${testid}"] .panel-title`);

    // ------------------------------------------------------------------
    // 1. Default panel — base styling only (grey background, bold title).
    // ------------------------------------------------------------------
    const defaultPanel = innerPanel('default-panel');
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).backgroundColor),
      'rgb(240, 240, 240)',
      'default panel must have base grey background',
    );
    const defaultTitle = innerPanelTitle('default-panel');
    assert.equal(
      await defaultTitle.evaluate((el) => getComputedStyle(el).fontWeight),
      '700',
      'default panel title must be bold (from base styles)',
    );
    // @media (min-width: 1px) always matches — letterSpacing should be applied.
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).letterSpacing),
      '1px',
      '@media inside variant must apply letterSpacing to .editor .panel',
    );
    // Nested at-rules: @supports (display: grid) wrapping @media — Chromium
    // supports grid so the outer condition holds, and the inner @media
    // (min-width: 1px) also holds, so the panel must have display:grid and
    // row-gap:12px. This exercises recursive at-rule scoping in
    // non_atomicify_rules.
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).display),
      'grid',
      'nested @supports must apply display:grid to .editor .panel',
    );
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).rowGap),
      '12px',
      '@media nested inside @supports must apply row-gap:12px to .editor .panel',
    );
    // Parent-pseudo selectors: verify `&:hover` and `&:focus` are correctly
    // flattened. We hover the element and check its cursor; we focus it and
    // check its outline-color. Both selectors are scoped under .cc-<hash>
    // and emitted from the postcss non_atomicify path that mirrors atomic.
    await defaultPanel.hover();
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).cursor),
      'pointer',
      '&:hover must apply cursor:pointer to .editor .panel',
    );
    await defaultPanel.focus();
    assert.equal(
      await defaultPanel.evaluate((el) => getComputedStyle(el).outlineColor),
      'rgb(0, 0, 255)',
      '&:focus must apply outline-color:rgb(0,0,255) to .editor .panel',
    );
    // Autoprefixer: user-select must produce a `-webkit-user-select` companion
    // declaration. We fetch the raw stylesheet source rather than reading
    // `CSSRule.cssText`, because browsers normalise CSSOM serialisation and
    // hide vendor prefixes for properties they recognise natively (both
    // unprefixed `user-select` and `-webkit-user-select` work in Chromium, so
    // the engine collapses them to a single representation).
    const rawCssTexts = await page.evaluate(async () => {
      const out: string[] = [];
      // Extracted mode: <link rel="stylesheet" href="...">. Fetch each href.
      for (const link of Array.from(
        document.querySelectorAll('link[rel="stylesheet"]'),
      ) as HTMLLinkElement[]) {
        if (link.href) {
          try {
            const res = await fetch(link.href);
            out.push(await res.text());
          } catch {
            /* unreachable */
          }
        }
      }
      // Runtime mode: <style> tags injected by `CS` runtime.
      for (const style of Array.from(document.querySelectorAll('style'))) {
        out.push(style.textContent ?? '');
      }
      return out.join('\n');
    });
    assert.match(
      rawCssTexts,
      /-webkit-user-select\s*:\s*none/,
      'autoprefixer must emit -webkit-user-select:none for cssMapScoped declarations',
    );
    assert.match(
      rawCssTexts,
      /(^|[^-])user-select\s*:\s*none/,
      'unprefixed user-select:none must also be present alongside the vendor prefix',
    );

    //
    // Deeper descendant nesting: .editor .panel .panel-footer must inherit
    // the scoped color, font-size, and margin from the nested rule.
    const defaultFooter = page.locator(
      '[data-testid="default-panel"] .panel-footer',
    );
    assert.equal(
      await defaultFooter.evaluate((el) => getComputedStyle(el).color),
      'rgb(100, 100, 100)',
      'nested .panel-footer rule must apply color:rgb(100,100,100)',
    );
    assert.equal(
      await defaultFooter.evaluate((el) => getComputedStyle(el).fontSize),
      '12px',
      'nested .panel-footer rule must apply font-size:12px',
    );
    assert.equal(
      await defaultFooter.evaluate((el) => getComputedStyle(el).marginTop),
      '6px',
      'nested .panel-footer rule must apply margin-top:6px',
    );

    // `& .panel-icon` inside `.editor .panel` — verifies that the inner `&`
    // refers to the OUTER descendant chain (not just the leaf `.panel`).
    // Expected flattened selector: `.cc-<hash> .editor .panel .panel-icon`.
    const defaultIcon = page.locator(
      `[data-testid="default-panel"] [data-testid="default-panel-icon"]`,
    );
    assert.equal(
      await defaultIcon.evaluate((el) => getComputedStyle(el).color),
      'rgb(170, 170, 170)',
      '`& .panel-icon` inside `.editor .panel` must flatten to .editor .panel .panel-icon and apply color',
    );

    // ------------------------------------------------------------------
    // 2. Danger panel — base + danger override (light red bg, red title).
    // ------------------------------------------------------------------
    const dangerPanel = innerPanel('danger-panel');
    assert.equal(
      await dangerPanel.evaluate((el) => getComputedStyle(el).backgroundColor),
      'rgb(255, 224, 224)',
      'danger panel must have light-red background (override wins over base)',
    );
    const dangerTitle = innerPanelTitle('danger-panel');
    assert.equal(
      await dangerTitle.evaluate((el) => getComputedStyle(el).color),
      'rgb(255, 0, 0)',
      'danger panel title must be red',
    );
    // Base styles must still apply where override doesn't set them.
    assert.equal(
      await dangerTitle.evaluate((el) => getComputedStyle(el).fontWeight),
      '700',
      'base bold title must still apply when override does not set fontWeight',
    );
    // Comma-separated selector list at the variant level —
    // `.editor .panel, .editor .panel-title { opacity: 0.95 }` must apply
    // the SAME opacity to BOTH targets. This catches regressions where
    // multi-selector rules accidentally get split or drop one side.
    assert.equal(
      await dangerPanel.evaluate((el) => getComputedStyle(el).opacity),
      '0.95',
      'comma-list selector must apply opacity:0.95 to .editor .panel',
    );
    assert.equal(
      await dangerTitle.evaluate((el) => getComputedStyle(el).opacity),
      '0.95',
      'comma-list selector must apply opacity:0.95 to .editor .panel-title (the second selector in the list)',
    );

    // ------------------------------------------------------------------
    // 3. New danger panel — base + old danger + new danger overrides stacked.
    // The new danger overrides the old danger's backgroundColor and color, and
    // adds an animation (from @keyframes pulse).
    // ------------------------------------------------------------------
    const dangerNewPanel = innerPanel('danger-new-panel');
    assert.equal(
      await dangerNewPanel.evaluate(
        (el) => getComputedStyle(el).backgroundColor,
      ),
      'rgb(255, 0, 0)',
      'new danger panel must have pure red background (newest override wins)',
    );
    const dangerNewTitle = innerPanelTitle('danger-new-panel');
    assert.equal(
      await dangerNewTitle.evaluate((el) => getComputedStyle(el).color),
      'rgb(255, 255, 255)',
      'new danger panel title must be white',
    );
    // Animation must be applied — animationName is set by panelDangerStylesNew
    // and references the keyframes identifier returned by `keyframes()`.
    const animationName = await dangerNewPanel.evaluate(
      (el) => getComputedStyle(el).animationName,
    );
    assert.notEqual(
      animationName,
      'none',
      `new danger panel must have an animation applied, got animationName: ${animationName}`,
    );

    // ------------------------------------------------------------------
    // 4. Each wrapper must carry a distinct `cc-<hash>` class name.
    // The `danger-panel` wrapper has two stacked variants and the
    // `danger-new-panel` wrapper has three — all the cc- class names must
    // be unique across the page (no collisions from shared variant keys).
    // ------------------------------------------------------------------
    const allCcClasses = await page.evaluate(() => {
      const classes = new Set();
      document
        .querySelectorAll('[data-testid]')
        .forEach((el) =>
          el.className
            .split(' ')
            .filter((c) => c.startsWith('cc-'))
            .forEach((c) => classes.add(c)),
        );
      return Array.from(classes);
    });

    // 3 wrappers × variants applied = at least 3 distinct cc- classes:
    //   default-panel:    panelStyles.default                 (1)
    //   danger-panel:     panelStyles.default + danger        (2)
    //   danger-new-panel: panelStyles.default + danger + new  (3)
    // The shared `panelStyles.default` only appears once → expect ≥ 3 unique.
    assert.ok(
      allCcClasses.length >= 3,
      `expected at least 3 distinct cc- classes across the page, got: ${allCcClasses.join(', ')}`,
    );

    // `panelDangerStyles.default` and `panelDangerStylesNew.default` share the
    // SAME variant key ("default") but live in different cssMapScoped bindings
    // — they MUST produce DIFFERENT class names (otherwise the new override's
    // CSS would collide with the old one).
    //
    // We verify this by checking that:
    //   - danger-panel has exactly 2 cc- classes (base + old danger override)
    //   - danger-new-panel has exactly 3 cc- classes (base + old + new override)
    //   - danger-new-panel has one cc- class that danger-panel doesn't (the new one)
    const dangerWrapperClasses = (
      await page.getByTestId('danger-panel').evaluate((el) => el.className)
    )
      .split(' ')
      .filter((c) => c.startsWith('cc-'));
    const dangerNewWrapperClasses = (
      await page.getByTestId('danger-new-panel').evaluate((el) => el.className)
    )
      .split(' ')
      .filter((c) => c.startsWith('cc-'));
    assert.equal(
      dangerWrapperClasses.length,
      2,
      `danger-panel must have exactly 2 cc- classes (base + danger), got: ${dangerWrapperClasses.join(',')}`,
    );
    assert.equal(
      dangerNewWrapperClasses.length,
      3,
      `danger-new-panel must have exactly 3 cc- classes (base + danger + new), got: ${dangerNewWrapperClasses.join(',')}`,
    );
    const newOnlyClasses = dangerNewWrapperClasses.filter(
      (c) => !dangerWrapperClasses.includes(c),
    );
    assert.equal(
      newOnlyClasses.length,
      1,
      `danger-new-panel must have exactly one cc- class that danger-panel doesn't (the new override), got: new=${dangerNewWrapperClasses.join(',')} old=${dangerWrapperClasses.join(',')}`,
    );

    // --------------------------------------------------------------------
    // 5. RTL panel — right-side ampersand i18n pattern.
    //   `[dir="rtl"] &`: { '.editor blockquote': { padding-left:0, padding-right:16 } }
    // The wrapper has `dir="rtl"`, which the `[dir="rtl"]` ancestor selector
    // matches. The blockquote inside should pick up RTL-flipped padding +
    // a coloured right border.
    // --------------------------------------------------------------------
    const rtlBlockquote = page.locator(
      `[data-testid="rtl-panel"] [data-testid="rtl-blockquote"]`,
    );
    assert.equal(
      await rtlBlockquote.evaluate((el) => getComputedStyle(el).paddingLeft),
      '0px',
      '`[dir="rtl"] &` rule must apply padding-left:0 to .editor blockquote inside an RTL wrapper',
    );
    assert.equal(
      await rtlBlockquote.evaluate((el) => getComputedStyle(el).paddingRight),
      '16px',
      '`[dir="rtl"] &` rule must apply padding-right:16px to .editor blockquote inside an RTL wrapper',
    );
    assert.equal(
      await rtlBlockquote.evaluate(
        (el) => getComputedStyle(el).borderRightColor,
      ),
      'rgb(0, 100, 200)',
      '`[dir="rtl"] &` rule must apply border-right-color to .editor blockquote inside an RTL wrapper',
    );
  }

  it('can bundle cssMapScoped with extract:true (sheets extracted to .css file)', async () => {
    // Extracted mode: @atlaspack/transformer-compiled-external pulls the
    // hoisted sheet strings out of the JS bundle, and @compiled/parcel-optimizer
    // links the resulting stylesheet into <head>. No runtime CSS injection.
    await assertCssMapScopedFixture('simple-project-with-css-map-scoped-extracted');
  });

  it('can bundle cssMapScoped with extract:false (sheets injected at runtime)', async () => {
    // Runtime mode: the CC/CS components inserted by the transform inject the
    // hoisted sheet strings via insertStyleSheet() at runtime. No separate
    // .css file is produced.
    await assertCssMapScopedFixture('simple-project-with-css-map-scoped-runtime');
  });
});
