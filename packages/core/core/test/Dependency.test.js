import expect from 'expect';
import {createDependencyId} from '../src/Dependency';
import {createEnvironment} from '../src/Environment';

describe('Dependency', () => {
  describe('createDependencyId', () => {
    it('should create a stable id for a dependency', () => {
      let id1 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
      });
      let id2 = createDependencyId({
        specifier: 'foo',
        env: createEnvironment(),
        specifierType: 'esm',
      });
      expect(id1).toEqual(id2);
    });
  });
});
