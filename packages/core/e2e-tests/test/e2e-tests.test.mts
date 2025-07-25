import assert from 'node:assert';
import {describe, it, before, after, beforeEach, afterEach} from 'node:test';
import {chromium} from 'playwright';
import type {Browser, Page, BrowserContext} from 'playwright';
import {buildFixture} from '../utils/build-fixture.mts';
import {serve} from '../utils/server.mts';
import type {ServeContext} from '../utils/server.mts';

describe('Atlaspack Playwright E2E tests', () => {
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

  it('can bundle a simple project', async () => {
    const filePath = await buildFixture('simple-project/index.html');
    server = await serve(filePath);

    await page.goto(server.address);
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');
  });

  it('can bundle a project with async imports', async () => {
    const filePath = await buildFixture('simple-project-with-async/index.html');
    server = await serve(filePath);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');
  });

  it('can bundle a project with asynchronous shared bundles', async () => {
    const filePath = await buildFixture(
      'simple-project-with-async-shared-bundles/index.html',
    );
    server = await serve(filePath);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
  });

  it('can bundle a project with shared bundles', async () => {
    const filePath = await buildFixture(
      'simple-project-with-shared-bundles/index.html',
    );
    server = await serve(filePath);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
  });
});
