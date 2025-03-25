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
    it('does not include packages not referenced in the config', () => {
      let unnecessaryDependencies = [];
      for (let dependency of packageJsonDependencyNames) {
        if (!configPackageReferences.has(dependency)) {
          unnecessaryDependencies.push(dependency);
        }
      }

      assert.deepEqual(unnecessaryDependencies, []);
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
