// @flow strict-local

import path from 'path';
import assert from 'assert';
import {chromium} from 'playwright';
import {Atlaspack} from '@atlaspack/core';

describe('Atlaspack Playwright E2E tests', () => {
  it('can bundle a simple project', async () => {
    const atlaspack = new Atlaspack({
      entries: [path.join(__dirname, 'data/simple-project/index.html')],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      serveOptions: {
        port: 1234,
      },
    });

    await atlaspack.run();
    await atlaspack.watch();

    const browser = await chromium.launch();
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto('http://localhost:1234');
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');

    await context.close();
    await browser.close();
  });

  it('can bundle a project with async imports', async () => {
    const atlaspack = new Atlaspack({
      entries: [
        path.join(__dirname, 'data/simple-project-with-async/index.html'),
      ],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      serveOptions: {
        port: 1234,
      },
    });

    await atlaspack.run();
    await atlaspack.watch();

    const browser = await chromium.launch();
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto('http://localhost:1234');
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, world!');

    await context.close();
    await browser.close();
  });

  it('can bundle a project with asynchronous shared bundles', async () => {
    const atlaspack = new Atlaspack({
      entries: [
        path.join(
          __dirname,
          'data/simple-project-with-async-shared-bundles/index.html',
        ),
      ],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      serveOptions: {
        port: 1234,
      },
    });

    await atlaspack.run();
    await atlaspack.watch();

    const browser = await chromium.launch();
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto('http://localhost:1234');
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');

    await context.close();
    await browser.close();
  });

  it('can bundle a project with shared bundles', async () => {
    const atlaspack = new Atlaspack({
      entries: [
        path.join(
          __dirname,
          'data/simple-project-with-shared-bundles/index.html',
        ),
      ],
      defaultConfig: require.resolve('@atlaspack/config-default'),
      serveOptions: {
        port: 1234,
      },
    });

    await atlaspack.run();
    await atlaspack.watch();

    const browser = await chromium.launch();
    const context = await browser.newContext();
    const page = await context.newPage();

    await page.goto('http://localhost:1234');
    const element = await page.getByTestId('content');
    assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');

    await context.close();
    await browser.close();
  });
});
