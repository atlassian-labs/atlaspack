// @flow

import assert from 'assert';

import config from '../';
import packageJson from '../package.json';

describe('@atlaspack/config-default', () => {
  let packageJsonDependencyNames: Set<string>;
  let configPackageReferences: Set<string>;

  before(() => {
    packageJsonDependencyNames = new Set(
      Object.keys(packageJson.dependencies || {}),
    );
    configPackageReferences = collectConfigPackageReferences(config);
  });

  describe('package.json', () => {
    it('config dependencies are in package.json', () => {
      assert.deepEqual(
        Array.from(packageJsonDependencyNames.values()).sort(),
        Array.from(configPackageReferences.values()).sort(),
      );
    });
  });
});

function collectConfigPackageReferences(
  configSection: mixed,
  references: Set<string> = new Set(),
): Set<string> {
  if (configSection == null || typeof configSection !== 'object') {
    throw new TypeError('Expected config section to be an object or an array');
  }

  for (let value of Object.values(configSection)) {
    if (typeof value === 'string') {
      if (value === '...') {
        continue;
      }

      references.add(value);
    } else if (configSection != null && typeof configSection === 'object') {
      collectConfigPackageReferences(value, references);
    } else {
      throw new Error(
        'Atlaspack configs must contain only strings, arrays, or objects in value positions',
      );
    }
  }

  return references;
}
