// @flow strict-local

import path from 'path';
import assert from 'assert';
import {chromium} from 'playwright';
import {Atlaspack} from '@atlaspack/core';

async function runPlaywrightTest(
  entry: string,
  // $FlowFixMe
  fn: (params: {|page: any|}) => Promise<void>,
): Promise<void> {
  const atlaspack = new Atlaspack({
    entries: [path.join(__dirname, 'data', entry)],
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

  await fn(page);

  await context.close();
  await browser.close();
}

describe('Atlaspack Playwright E2E tests', () => {
  it('can bundle a simple project', async () => {
    await runPlaywrightTest('simple-project/index.html', async ({page}) => {
      const element = await page.getByTestId('content');
      assert.equal(await element.innerText(), 'Hello, world!');
    });
  });

  it('can bundle a project with async imports', async () => {
    await runPlaywrightTest(
      'simple-project-with-async/index.html',
      async ({page}) => {
        const element = await page.getByTestId('content');
        assert.equal(await element.innerText(), 'Hello, world!');
      },
    );
  });

  it('can bundle a project with asynchronous shared bundles', async () => {
    await runPlaywrightTest(
      'simple-project-with-async-shared-bundles/index.html',
      async ({page}) => {
        const element = await page.getByTestId('content');
        assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
      },
    );
  });

  it('can bundle a project with shared bundles', async () => {
    await runPlaywrightTest(
      'simple-project-with-shared-bundles/index.html',
      async ({page}) => {
        const element = await page.getByTestId('content');
        assert.equal(await element.innerText(), 'Hello, CRAZY WORLD!');
      },
    );
  });
});
