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

  it('can bundle a project that uses cssMap natively', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-compiled-css-in-js-natively-css-map/index.html',
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
          'data/simple-project-with-compiled-css-in-js-natively-css-map/.atlaspackrc',
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

    // Verify the default hash strategy produces 9-char class names (e.g. _syaz5scu).
    const className = await headingElement.getAttribute('class');
    assert.ok(
      className && /\b_[a-zA-Z0-9]{8}\b/.test(className),
      `Expected a 9-char default-strategy class name, got: ${className}`,
    );

    const buttonElement = page.getByTestId('button');
    const cursor = await buttonElement.evaluate(
      (el) => getComputedStyle(el).cursor,
    );
    assert.equal(cursor, 'pointer');
  });

  it('can bundle a project that uses cssMap with hashStrategy "max" natively', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-compiled-css-in-js-natively-css-map-hash-strategy/index.html',
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
          'data/simple-project-with-compiled-css-in-js-natively-css-map-hash-strategy/.atlaspackrc',
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

    // Verify the "max" hash strategy produces 11-char class names (6-char group + 4-char value).
    const className = await headingElement.getAttribute('class');
    assert.ok(
      className && /\b_[a-zA-Z0-9]{10}\b/.test(className),
      `Expected an 11-char max-strategy class name, got: ${className}`,
    );

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
});
