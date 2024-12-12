// @flow
import expect from 'expect';
import {createDependencyId} from '../src/Dependency';
import {createEnvironment} from '../src/Environment';
import type {ProjectPath} from '../src/projectPath';

describe('Dependency', () => {
  describe('createDependencyId', () => {
    it('should create a stable id for a dependency', () => {
      let id1 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        bundleBehavior: undefined,
        packageConditions: undefined,
        pipeline: undefined,
        priority: undefined,
        sourceAssetId: undefined,
        target: undefined,
      });
      let id2 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        bundleBehavior: undefined,
        packageConditions: undefined,
        pipeline: undefined,
        priority: undefined,
        sourceAssetId: undefined,
        target: undefined,
      });
      expect(id1).toEqual(id2);
    });

    it('dependencies with different targets should have different IDs', () => {
      let id1 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        target: {
          name: 'test-1234',
          distDir: (('dist-dir': any): ProjectPath),
          env: createEnvironment(),
          publicUrl: 'public-url',
          source: '1234',
        },
        bundleBehavior: undefined,
        packageConditions: undefined,
        pipeline: undefined,
        priority: undefined,
        sourceAssetId: undefined,
      });
      let id2 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
        target: {
          name: 'test-1234',
          distDir: (('dist-dir': any): ProjectPath),
          env: createEnvironment(),
          publicUrl: 'public-url',
          source: '5678', // <- this is different
        },
        bundleBehavior: undefined,
        packageConditions: undefined,
        pipeline: undefined,
        priority: undefined,
        sourceAssetId: undefined,
      });
      expect(id1).not.toEqual(id2);
    });
  });
});
