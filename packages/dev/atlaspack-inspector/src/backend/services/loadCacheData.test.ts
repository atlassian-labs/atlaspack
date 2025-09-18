import path from 'path';
import {TemporaryDirectory} from '../testing/TemporaryDirectory';
import Atlaspack from '@atlaspack/core';
import fs from 'fs';
import {loadCacheData} from './loadCacheData';
import assert from 'assert';

jest.mock('../config/logger');

async function setupMockProject(): Promise<TemporaryDirectory> {
  const tempDir = new TemporaryDirectory();

  await fs.promises.writeFile(path.join(tempDir.get(), '.git'), '', 'utf-8');
  await fs.promises.writeFile(
    path.join(tempDir.get(), '.parcelrc'),
    JSON.stringify({
      extends: '@atlaspack/config-default',
    }),
    'utf-8',
  );
  await fs.promises.writeFile(
    path.join(tempDir.get(), 'index.js'),
    `
import './a.js';
import './b.js';
      `,
    'utf-8',
  );
  await fs.promises.writeFile(
    path.join(tempDir.get(), 'a.js'),
    'console.log("Hello, world!");',
    'utf-8',
  );
  await fs.promises.writeFile(
    path.join(tempDir.get(), 'b.js'),
    'console.log("Hello, world!");',
    'utf-8',
  );

  const atlaspack = new Atlaspack({
    featureFlags: {
      cachePerformanceImprovements: true,
    },
    entries: [path.join(tempDir.get(), 'index.js')],
  });
  await atlaspack.run();

  return tempDir;
}

describe('loadCacheData', () => {
  it('should load the cache data', async () => {
    const tempDir = await setupMockProject();
    const cacheData = await loadCacheData({
      target: tempDir.get(),
      projectRoot: tempDir.get(),
      repositoryRoot: tempDir.get(),
    });

    const assetGraph = cacheData.assetGraph;
    let numAssets = 0;
    assetGraph.traverseAssets(() => {
      numAssets++;
    });
    assert.equal(numAssets, 3);

    const treemap = cacheData.treemap;
    const assetTree = treemap?.bundles[0].assetTree;

    const cleanNode = (node: any) => {
      delete node.id;
      Object.values(node.children).forEach(cleanNode);
    };

    cleanNode(assetTree);

    expect(assetTree).toMatchInlineSnapshot(`
     {
       "children": {
         "a.js": {
           "children": {},
           "path": "/a.js",
           "size": 30,
         },
         "b.js": {
           "children": {},
           "path": "/b.js",
           "size": 30,
         },
         "index.js": {
           "children": {},
           "path": "/index.js",
           "size": 60,
         },
       },
       "path": "",
       "size": 120,
     }
    `);
  });
});
