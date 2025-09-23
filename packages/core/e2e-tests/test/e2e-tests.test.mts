import assert from 'node:assert';
import {describe, it, before, after, beforeEach, afterEach} from 'node:test';
import {chromium} from 'playwright';
import type {Browser, Page, BrowserContext} from 'playwright';
import {buildFixture, serveFixture} from '../utils/build-fixture.mts';
import {serve} from '../utils/server.mts';
import type {ServeContext} from '../utils/server.mts';
import {writeFile} from 'node:fs/promises';
import {basename, join} from 'node:path';
import nullthrows from 'nullthrows';

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
    const {outputDir} = await buildFixture('simple-project/index.html');
    server = await serve(outputDir);

    await page.goto(`${server.address}`);
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');
  });

  it('can serve a simple project', async () => {
    server = await serveFixture('simple-project/index.html');
    await page.goto(`${server.address}/index.html`);
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');
  });

  it('can bundle a project with async imports', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-async/index.html',
    );
    server = await serve(outputDir);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');
  });

  it('can bundle a project with asynchronous shared bundles', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-async-shared-bundles/index.html',
    );
    server = await serve(outputDir);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
  });

  it('can bundle a project with shared bundles', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-shared-bundles/index.html',
    );
    server = await serve(outputDir);
    await page.goto(server.address);

    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
  });

  it('can bundle a project with conditional bundles', async () => {
    const {outputDir, buildResult} = await buildFixture(
      'simple-project-with-conditional-bundles/index.html',
      {
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
        featureFlags: {
          conditionalBundlingApi: true,
        },
      },
    );

    const a = nullthrows(
      buildResult.bundleGraph
        .getBundles()
        .find((b) => b.displayName === 'a.[hash].js'),
      'a.js bundle not found',
    );

    const index = nullthrows(
      buildResult.bundleGraph
        .getBundles()
        .find((b) => b.displayName === 'index.[hash].js'),
      'index.js bundle not found',
    );

    await writeFile(
      join(outputDir, 'server-mock.html'),
      `
      <div data-testid="content" id="output"></div>
      <script src="/${a.filePath}" type="module"></script>
      <script src="/${index.filePath}" type="module"></script>
      `,
    );

    await writeFile(
      join(outputDir, 'server-mock.html'),
      `
      <div data-testid="content" id="output"></div>
      <script src="/${basename(a.filePath)}" type="module"></script>
      <script src="/${basename(index.filePath)}" type="module"></script>
      `,
    );

    server = await serve(outputDir);
    await page.goto(`${server.address}/server-mock.html`);

    const element = page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, module-a!');
  });

  it('can bundle a project with conditional bundles and fallback safely', async () => {
    const {outputDir} = await buildFixture(
      'simple-project-with-conditional-bundles/index.html',
      {
        mode: 'production',
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
        featureFlags: {
          conditionalBundlingApi: true,
          condbDevFallbackProd: true,
        },
      },
    );

    server = await serve(outputDir);
    await page.goto(server.address);

    const element = page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, module-a!');

    const atlaspackErrors = await page.evaluate(
      () => globalThis.__ATLASPACK_ERRORS,
    );
    assert.ok(
      atlaspackErrors?.[0]?.message?.includes(
        'Sync dependency fallback triggered for condition "cond1": Cannot find module',
      ),
    );
  });
});
