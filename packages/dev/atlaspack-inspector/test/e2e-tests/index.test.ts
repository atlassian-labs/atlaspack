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
    for (const link of links) {
      const href = await link.getAttribute('href');
      // Match only `/app/cache/<key>` style links. The previous regex
      // `app\/cache\/.+` also matched sibling routes like `/app/cache-stats`
      // and `/app/cache-invalidation-files`, which could be picked first on
      // CI (depending on link ordering) and would navigate away from a real
      // cache entry, causing the "Cache entry" content assertion to fail.
      if (href && /^\/?app\/cache\/[^/]+\/?$/.test(href)) {
        await Promise.all([page.waitForURL(/\/app\/cache\/.+/), link.click()]);
        await page.waitForLoadState('networkidle');
        await page.waitForSelector('atlaspack-inspector-loading-indicator', {
          state: 'detached',
        });
        // Explicitly wait for the cache entry view to render before asserting
        // on body content. Without this, CI can race: `networkidle` may fire
        // before the SPA finishes mounting the cache-entry view, causing the
        // assertion to read stale (cache-list) content.
        await page
          .getByText('Cache entry size', {exact: false})
          .first()
          .waitFor({timeout: 10000});

        const text = await page.textContent('body');
        assert.ok(
          text?.includes('Cache entry'),
          'Failed to find cache entry content',
        );
        assert.ok(
          text?.includes('Cache entry size'),
          'Failed to find cache entry code on cache entry',
        );
        await expect(page).toHaveScreenshot('cache-entry.png', {
          maxDiffPixelRatio: 0.05,
        });

        return;
      }
    }

    throw new Error('Failed to find cache entry link');
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
