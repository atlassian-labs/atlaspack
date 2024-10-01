import React, {lazy, Suspense, useState} from 'react';
import ReactDOM from 'react-dom';
import RegularExport from './regular-import';
console.log(RegularExport);

const Feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');
const FeatureWithUI = importCond<
// @ts-expect-error - TS6142 - Module './feature-ui-enabled' was resolved to '/home/ubuntu/parcel/packages/examples/conditional-bundling/src/feature-ui-enabled.tsx', but '--jsx' is not set.
  typeof import('./feature-ui-enabled'),
// @ts-expect-error - TS6142 - Module './feature-ui-disabled' was resolved to '/home/ubuntu/parcel/packages/examples/conditional-bundling/src/feature-ui-disabled.tsx', but '--jsx' is not set.
  typeof import('./feature-ui-disabled')
>('feature.ui', './feature-ui-enabled', './feature-ui-disabled');

// @ts-expect-error - TS6142 - Module './lazy-component' was resolved to '/home/ubuntu/parcel/packages/examples/conditional-bundling/src/lazy-component.tsx', but '--jsx' is not set.
const LazyComponent = lazy(() => import('./lazy-component'));

function LazyComponentContainer() {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. | TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <Suspense fallback={<p>Loading...</p>}>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <LazyComponent />
    </Suspense>
  );
}

const App = () => {
  const [showLazyComponent, setShowLazyComponent] = useState(false);

  console.log('FeatureWithUI', FeatureWithUI);
  console.log('Feature', Feature);
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <div>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <p>Hello from React</p>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <button onClick={() => setShowLazyComponent(!showLazyComponent)}>
        Toggle lazy component
      </button>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <p>Conditional Feature: {Feature()}</p>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      <FeatureWithUI />
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      {showLazyComponent ? <LazyComponentContainer /> : null}
    </div>
  );
};

// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
ReactDOM.render(<App />, document.getElementById('container'));
