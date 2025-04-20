// @flow strict-local

import assert from 'assert';
import {createEnvironment} from '../Environment';
import {createDependency} from '../Dependency';
import {getPublicDependency} from '../public/Dependency';
import {DEFAULT_OPTIONS} from './test-utils';

describe('Public Dependency', () => {
  it('returns the same public Dependency given an internal dependency', () => {
    let internalDependency = createDependency('/', {
      specifier: 'foo',
      specifierType: 'esm',
      env: createEnvironment({}),
    });

    assert.equal(
      getPublicDependency(internalDependency, DEFAULT_OPTIONS),
      getPublicDependency(internalDependency, DEFAULT_OPTIONS),
    );
  });
});
