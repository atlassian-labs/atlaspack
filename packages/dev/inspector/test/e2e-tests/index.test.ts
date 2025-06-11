import Atlaspack from '@atlaspack/core';
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
    const atlaspack = new Atlaspack({
      entries: [path.join(__dirname, 'mock-project', 'index.js')],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      cacheDir: path.join(__dirname, 'mock-project', '.atlaspack-cache'),
      distDir: path.join(__dirname, 'mock-project', 'dist'),
      featureFlags: {
        cachePerformanceImprovements: true,
      },
    });

    await atlaspack.run();

    const inspectorAppParams = await configureInspectorApp({
      target: path.join(__dirname, 'mock-project'),
    });
    app = buildInspectorApp(inspectorAppParams);
    await new Promise((resolve) => {
      server = app.listen(0, () => resolve(null));
    });

    port = (server.address() as AddressInfo).port;
  });

  let browser: Browser;
  let context: BrowserContext;
  let page: Page;

  test.beforeAll(async () => {
    browser = await chromium.launch();
    context = await browser.newContext();
    page = await context.newPage();
  });

  test('can load the home page with stats', async () => {
    await page.goto(`http://localhost:${port}`);
    await page.waitForLoadState('networkidle');

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
      if (href && /app\/cache\/.+/.test(href)) {
        await link.click();
        await page.waitForLoadState('networkidle');
        const text = await page.textContent('body');
        assert.ok(
          text?.includes('Cache entry'),
          'Failed to find cache entry content',
        );
        assert.ok(
          text?.includes('Hello, world!'),
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
    });
  });
});
