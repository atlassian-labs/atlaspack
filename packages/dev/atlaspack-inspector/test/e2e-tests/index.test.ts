import Atlaspack from '@atlaspack/core';
import fs from 'fs';
import path from 'path';
import http from 'http';
import express from 'express';
import {Browser, BrowserContext, chromium, Page} from 'playwright';
import {test, expect} from '@playwright/test';
import {
  configureInspectorApp,
  buildInspectorApp,
} from '../../src/backend/index';
import {AddressInfo} from 'net';
import assert from 'assert';

test.describe('Atlaspack Inspector E2E tests', () => {
  let app: express.Express;
  let server: http.Server;
  let port: number;

  test.beforeAll(async () => {
    fs.rmSync(path.join(__dirname, 'mock-project', '.atlaspack-cache'), {
      recursive: true,
      force: true,
    });

    const atlaspack = new Atlaspack({
      entries: [path.join(__dirname, 'mock-project', 'index.js')],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      cacheDir: path.join(__dirname, 'mock-project', '.atlaspack-cache'),
    });
    await atlaspack.run();

    const inspectorAppParams = configureInspectorApp({
      target: path.join(__dirname, 'mock-project'),
      projectRoot: path.join(__dirname, 'mock-project'),
    });
    app = buildInspectorApp(inspectorAppParams);
    await new Promise((resolve) => {
      server = app.listen(0, () => resolve(null));
    });

    port = (server.address() as AddressInfo).port;

    const cacheData = await inspectorAppParams.cacheData;
    // Force stuff to be loaded
    assert.ok(cacheData.assetGraph);
    assert.ok(cacheData.bundleGraph);
    assert.ok(cacheData.treemap);
  });

  let browser: Browser;
  let context: BrowserContext;
  let page: Page;

  test.beforeAll(async () => {
    browser = await chromium.launch({
      headless: true,
    });
    context = await browser.newContext();
    page = await context.newPage();
  });

  test('can load the home page with stats', async () => {
    await page.goto(`http://localhost:${port}/app/cache-stats`);
    await page.waitForLoadState('networkidle');
    await page.waitForSelector('atlaspack-inspector-loading-indicator', {
      state: 'detached',
    });

    const title = await page.title();
    assert.equal(title, 'Atlaspack Inspector');

    const h1 = await page.$('h1');
    assert.ok(h1);
    const text = await h1?.textContent();
    assert.equal(text, 'Atlaspack cache stats');

    await expect(page).toHaveScreenshot('home.png', {
      maxDiffPixelRatio: 0.05,
    });
  });

  test('can load the cache list and entry pages', async () => {
    await page.goto(`http://localhost:${port}/app/cache`);
    await page.waitForLoadState('networkidle');
    await page.waitForSelector('atlaspack-inspector-loading-indicator', {
      state: 'detached',
    });

    const text = await page.textContent('body');
    assert.ok(
      text?.includes('Click a cache key to view its contents'),
      'Failed to find cache list content',
    );
    await expect(page).toHaveScreenshot('cache-list.png', {
      maxDiffPixelRatio: 0.05,
    });

    const links = await page.$$('a');
    // Fetch all hrefs in parallel, then find the first real cache entry link.
    // Match only `/app/cache/<key>` style links — not sibling routes like
    // `/app/cache-stats` or `/app/cache-invalidation-files`.
    const hrefs = await Promise.all(links.map((l) => l.getAttribute('href')));
    const cacheEntryIndex = hrefs.findIndex(
      (href) => href && /^\/?app\/cache\/[^/]+\/?$/.test(href),
    );

    if (cacheEntryIndex === -1) {
      throw new Error('Failed to find cache entry link');
    }

    await Promise.all([
      page.waitForURL(/\/app\/cache\/.+/),
      links[cacheEntryIndex].click(),
    ]);
    // Wait for the cache entry view to fully render. This is the authoritative
    // readiness signal — it replaces separate networkidle / loading-indicator
    // waits which could resolve before the SPA finishes mounting the view.
    await page
      .getByText('Cache entry size', {exact: false})
      .first()
      .waitFor({timeout: 10000});

    const entryText = await page.textContent('body');
    assert.ok(
      entryText?.includes('Cache entry size'),
      'Failed to find cache entry content',
    );
    await expect(page).toHaveScreenshot('cache-entry.png', {
      maxDiffPixelRatio: 0.05,
    });
  });

  test('can load the treemap', async function () {
    await page.goto(`http://localhost:${port}/app/treemap`);
    await page.waitForLoadState('networkidle');

    await expect(page).toHaveScreenshot('treemap.png', {
      // The graph visualiser and treemap are both non-deterministic
      // so we need to allow for some diffs
      maxDiffPixelRatio: 0.25,
      timeout: 10000,
    });
  });
});
