// @flow
import expect from 'expect';
import {createDependencyId} from '../src/Dependency';
import {createEnvironment} from '../src/Environment';

describe('Dependency', () => {
  describe('createDependencyId', () => {
    it('should create a stable id for a dependency', () => {
      // $FlowFixMe missing properties
      let id1 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
      });
      // $FlowFixMe missing properties
      let id2 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
      });
      expect(id1).toEqual(id2);
    });

    it('dependencies with different targets should have different IDs', () => {
      // $FlowFixMe missing properties
      let id1 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        // $FlowFixMe missing properties
        target: {
          name: 'test-1234',
          distDir: 'dist-dir',
          env: createEnvironment(),
          publicUrl: 'public-url',
          source: '1234',
        },
      });
      // $FlowFixMe missing properties
      let id2 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        // $FlowFixMe missing properties
        target: {
          name: 'test-1234',
          distDir: 'dist-dir',
          env: createEnvironment(),
          publicUrl: 'public-url',
          source: '5678', // <- this is different
        },
      });
      expect(id1).not.toEqual(id2);
    });
  });
});
