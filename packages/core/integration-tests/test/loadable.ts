import path from 'path';
import assert from 'assert';
import {fsFixture, overlayFS, bundle, findAsset} from '@atlaspack/test-utils';

describe('loadable side effects', () => {
  it('should mark ancestors of loadable package as having side effects', async () => {
    await fsFixture(overlayFS, __dirname)`
      package.json:
        { "sideEffects": false }
      index.jsx:
        import {LoadableComponent} from './loadable-component';
        import {nonLoadableFn} from './non-loadable-fn';
        export default () => <LoadableComponent />;

      loadable-component.js:
        import {loadable} from '@confluence/loadable';
        export const LoadableComponent = loadable(() => import('./TestingComponent'));

      TestingComponent.jsx:
        export default () => <div>Testing Component</div>;

      non-loadable-fn.js:
        export const nonLoadableFn = () => 'non-loadable-fn';

      node_modules/@confluence/loadable/index.js:
        export const loadable = (inputComponent) => inputComponent;
    `;

    let result = await bundle(path.join(__dirname, 'index.jsx'), {
      inputFS: overlayFS,
      featureFlags: {
        loadableSideEffects: true,
      },
    });

    let asset = findAsset(result, 'index.jsx');
    assert.equal(
      asset?.sideEffects,
      true,
      'sideEffects should be true for index.js',
    );

    let loadableComponentAsset = findAsset(result, 'loadable-component.js');
    assert.equal(
      loadableComponentAsset?.sideEffects,
      true,
      'sideEffects should be true for loadable-component.js',
    );

    // The non-loadable file is not an ancestor of @confluence/loadable, so
    // should not be marked as having side effects.
    let nonLoadableAsset = findAsset(result, 'non-loadable-fn.js');
    assert.equal(
      nonLoadableAsset?.sideEffects,
      false,
      'sideEffects should be false for non-loadable-fn.js',
    );

    // The TestingComponent is downstream of the loadable component, so should
    // not have side effects.
    let testingComponentAsset = findAsset(result, 'TestingComponent.jsx');
    assert.equal(
      testingComponentAsset?.sideEffects,
      false,
      'sideEffects should be true for TestingComponent.jsx',
    );
  });
});
