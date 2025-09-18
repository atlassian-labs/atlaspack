import assert from 'assert';
import {getDisplayName} from './getDisplayName';

describe('getDisplayName', function () {
  it('should return display name for asset node', function () {
    const assetNode = {
      type: 'asset',
      value: {
        filePath: '/path/to/file.js',
      },
      id: 'asset1',
    };

    const result = getDisplayName(assetNode);

    assert.equal(result, 'asset: /path/to/file.js');
  });

  it('should return display name for dependency node', function () {
    const dependencyNode = {
      type: 'dependency',
      value: {
        specifier: './module',
      },
      id: 'dep1',
    };

    const result = getDisplayName(dependencyNode);

    assert.equal(result, "dependency: import './module'");
  });

  it('should return display name for asset_group node', function () {
    const assetGroupNode = {
      type: 'asset_group',
      value: {
        filePath: '/path/to/asset-group.js',
      },
      id: 'group1',
    };

    const result = getDisplayName(assetGroupNode);

    assert.equal(result, 'asset group: /path/to/asset-group.js');
  });

  it('should return display name for bundle node', function () {
    const bundleNode = {
      type: 'bundle',
      value: {
        displayName: 'main.js',
      },
      id: 'bundle1',
    };

    const result = getDisplayName(bundleNode);

    assert.equal(result, 'bundle: main.js');
  });

  it('should return node id for unknown node type', function () {
    const unknownNode = {
      type: 'unknown',
      value: {},
      id: 'unknown123',
    };

    const result = getDisplayName(unknownNode);

    assert.equal(result, 'unknown123');
  });

  it('should return node id when value is null', function () {
    const nodeWithNullValue = {
      type: 'unknown_type',
      value: null,
      id: 'null-asset',
    };

    const result = getDisplayName(nodeWithNullValue);

    assert.equal(result, 'null-asset');
  });
});
