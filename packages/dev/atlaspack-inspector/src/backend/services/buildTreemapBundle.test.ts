import assert from 'assert';
import * as sinon from 'sinon';
import fs from 'fs';
import path from 'path';
import {buildTreemapBundle} from './buildTreemap';

// Explicit mock required: Jest's automocking doesn't properly handle named const exports
// with TypeScript's "module": "NodeNext" configuration. See loadCacheData.test.ts for details.
jest.mock('../config/logger', () => ({
  logger: {
    info: jest.fn(),
    debug: jest.fn(),
    error: jest.fn(),
    warn: jest.fn(),
  },
}));

describe('buildTreemapBundle', function () {
  let sandbox: sinon.SinonSandbox;
  let mockBundleGraph: any;
  let mockNode: any;
  let mockWriteBundleRequestsByBundleId: Map<string, any>;

  beforeEach(() => {
    sandbox = sinon.createSandbox();

    // Mock fs.statSync
    sandbox.stub(fs, 'statSync').returns({size: 1024} as any);

    // Mock path.join
    sandbox
      .stub(path, 'join')
      .callsFake((...paths: string[]) => paths.join('/'));

    // Mock bundle node
    mockNode = {
      id: 'bundle123',
      type: 'bundle',
      value: {
        displayName: 'main.js',
      },
    };

    // Mock bundle graph with traverseAssets method
    mockBundleGraph = {
      traverseAssets: sandbox.stub(),
    };

    // Mock write bundle requests map
    mockWriteBundleRequestsByBundleId = new Map([
      [
        'bundle123',
        {
          result: {
            bundleId: 'bundle123',
            filePath: 'dist/main.js',
          },
        },
      ],
    ]);
  });

  afterEach(() => {
    sandbox.restore();
  });

  it('should build treemap bundle with correct basic properties', function () {
    // Setup mock assets
    const mockAssets = [
      {
        filePath: '/src/index.js',
        stats: {size: 100},
      },
      {
        filePath: '/src/utils.js',
        stats: {size: 50},
      },
    ];

    // Mock traverseAssets to call callback with each asset
    mockBundleGraph.traverseAssets.callsFake(
      (bundleValue: any, callback: (asset: any) => void) => {
        mockAssets.forEach(callback);
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    assert.equal(result.id, 'bundle123');
    assert.equal(result.displayName, 'main.js');
    assert.equal(result.bundle, mockNode);
    assert.equal(result.size, 1024); // from mocked fs.statSync
    assert.equal(result.filePath, 'dist/main.js');
    assert(result.assetTree);
  });

  it('should handle missing write bundle request', function () {
    const emptyMap = new Map();

    mockBundleGraph.traverseAssets.callsFake(
      (_bundleValue: any, _callback: (asset: any) => void) => {
        // No assets
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: emptyMap,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    assert.equal(result.size, 0);
    assert.equal(result.filePath, '');
  });

  it('should build correct asset tree structure', function () {
    const mockAssets = [
      {
        filePath: 'src/components/Button.js',
        stats: {size: 100},
      },
      {
        filePath: 'src/utils/helpers.js',
        stats: {size: 50},
      },
    ];

    mockBundleGraph.traverseAssets.callsFake(
      (bundleValue: any, callback: (asset: any) => void) => {
        mockAssets.forEach(callback);
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    // Check that tree structure exists
    assert(result.assetTree.children.src);
    assert.equal(result.assetTree.path, '');
    assert.equal(result.assetTree.children.src.path, '/src');

    // Check components subdirectory exists
    assert(result.assetTree.children.src.children.components);
    assert.equal(
      result.assetTree.children.src.children.components.path,
      '/src/components',
    );

    // Check utils subdirectory exists
    assert(result.assetTree.children.src.children.utils);
    assert.equal(
      result.assetTree.children.src.children.utils.path,
      '/src/utils',
    );

    assert.equal(result.assetTree.size, 150);
    assert.equal(result.assetTree.children.src.size, 150);
    assert.equal(result.assetTree.children.src.children.components.size, 100);
    assert.equal(result.assetTree.children.src.children.utils.size, 50);
  });

  it('should handle assets with same directory correctly', function () {
    const mockAssets = [
      {
        filePath: 'lib/module1.js',
        stats: {size: 100},
      },
      {
        filePath: 'lib/module2.js',
        stats: {size: 200},
      },
    ];

    mockBundleGraph.traverseAssets.callsFake(
      (bundleValue: any, callback: (asset: any) => void) => {
        mockAssets.forEach(callback);
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    // lib directory should contain total of all files under it
    assert.equal(result.assetTree.children.lib.size, 300); // 100 + 200
    assert.equal(
      result.assetTree.children.lib.children['module1.js'].size,
      100,
    );
    assert.equal(
      result.assetTree.children.lib.children['module2.js'].size,
      200,
    );
  });

  it('should handle empty assets array', function () {
    mockBundleGraph.traverseAssets.callsFake(
      (_bundleValue: any, _callback: (asset: any) => void) => {
        // No assets
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    assert.equal(result.assetTree.size, 0);
    assert.deepEqual(result.assetTree.children, {});
  });

  it('should handle file path without extension', function () {
    const mockAssets = [
      {
        filePath: 'src/config',
        stats: {size: 25},
      },
    ];

    mockBundleGraph.traverseAssets.callsFake(
      (_bundleValue: any, callback: (asset: any) => void) => {
        mockAssets.forEach(callback);
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    assert.equal(result.assetTree.children.src.children.config.size, 25);
  });

  it('should call traverseAssets with correct bundle value', function () {
    mockBundleGraph.traverseAssets.callsFake(
      (_bundleValue: any, _callback: (asset: any) => void) => {
        // Just verify it was called
      },
    );

    buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    assert(mockBundleGraph.traverseAssets.calledOnce);
    assert(mockBundleGraph.traverseAssets.calledWith(mockNode.value));
  });

  it('should calculate directory sizes correctly', function () {
    // Test case matching user's example:
    // lib/child/a.js - 10
    // lib/other/b.js - 10
    // lib/other/c.js - 10
    const mockAssets = [
      {
        filePath: 'lib/child/a.js',
        stats: {size: 10},
      },
      {
        filePath: 'lib/other/b.js',
        stats: {size: 10},
      },
      {
        filePath: 'lib/other/c.js',
        stats: {size: 10},
      },
    ];

    mockBundleGraph.traverseAssets.callsFake(
      (bundleValue: any, callback: (asset: any) => void) => {
        mockAssets.forEach(callback);
      },
    );

    const result = buildTreemapBundle({
      writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
      node: mockNode,
      projectRoot: '/project',
      repositoryRoot: '/project',
      bundleGraph: mockBundleGraph,
    });

    // Root should contain total of all files
    // assert.equal(result.assetTree.size, 30); // 10+10+10

    // lib directory should contain total of all files under lib
    assert.equal(result.assetTree.children.lib.size, 30); // 10+10+10

    // lib/child should contain only a.js
    assert.equal(result.assetTree.children.lib.children.child.size, 10); // 10

    // lib/other should contain b.js + c.js
    assert.equal(result.assetTree.children.lib.children.other.size, 20); // 10+10

    // Individual files should have their own sizes
    assert.equal(
      result.assetTree.children.lib.children.child.children['a.js'].size,
      10,
    );
    assert.equal(
      result.assetTree.children.lib.children.other.children['b.js'].size,
      10,
    );
    assert.equal(
      result.assetTree.children.lib.children.other.children['c.js'].size,
      10,
    );
  });

  it('should handle fs.statSync errors gracefully', function () {
    // Make fs.statSync throw an error
    (fs.statSync as sinon.SinonStub).throws(new Error('File not found'));

    mockBundleGraph.traverseAssets.callsFake(
      (_bundleValue: any, _callback: (asset: any) => void) => {
        // No assets
      },
    );

    assert.doesNotThrow(() => {
      buildTreemapBundle({
        writeBundleRequestsByBundleId: mockWriteBundleRequestsByBundleId,
        node: mockNode,
        projectRoot: '/project',
        repositoryRoot: '/project',
        bundleGraph: mockBundleGraph,
      });
    }, Error);
  });
});
