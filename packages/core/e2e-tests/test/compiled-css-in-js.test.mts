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
});
