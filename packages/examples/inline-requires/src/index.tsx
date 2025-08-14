import React, {lazy, Suspense, useState} from 'react';
import ReactDOM from 'react-dom';
import RegularExport from './regular-import';
console.log(RegularExport);

// @ts-expect-error TS2304
const Feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');
console.log('Feature', Feature);

// @ts-expect-error TS2304
const FeatureWithUI = importCond<
  // @ts-expect-error TS6142
  typeof import('./feature-ui-enabled'),
  // @ts-expect-error TS6142
  typeof import('./feature-ui-disabled')
>('feature.ui', './feature-ui-enabled', './feature-ui-disabled');
console.log('FeatureWithUI', FeatureWithUI);

// @ts-expect-error TS6142
const LazyComponent = lazy(() => import('./lazy-component'));

function LazyComponentContainer() {
  return (
    // @ts-expect-error TS17004
    <Suspense fallback={<p>Loading...</p>}>
      {/*
       // @ts-expect-error TS17004 */}
      <LazyComponent />
    </Suspense>
  );
}

const App = () => {
  const [showLazyComponent, setShowLazyComponent] = useState(false);

  return (
    // @ts-expect-error TS17004
    <div>
      {/*
       // @ts-expect-error TS17004 */}
      <p>Hello from React</p>
      {/*
       // @ts-expect-error TS17004 */}
      <button onClick={() => setShowLazyComponent(!showLazyComponent)}>
        Toggle lazy component
      </button>
      {/*
       // @ts-expect-error TS17004 */}
      <p>Conditional Feature: {Feature()}</p>
      {/*
       // @ts-expect-error TS17004 */}
      <FeatureWithUI />
      {/*
       // @ts-expect-error TS17004 */}
      {showLazyComponent ? <LazyComponentContainer /> : null}
    </div>
  );
};

// @ts-expect-error TS17004
ReactDOM.render(<App />, document.getElementById('container'));
