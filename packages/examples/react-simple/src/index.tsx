import React, {lazy, Suspense, useState} from 'react';
import ReactDOM from 'react-dom';
import RegularExport from './regular-import';
console.log(RegularExport);

import Feature from './feature-enabled';
console.log('Feature', Feature);

import FeatureWithUI from './feature-ui-enabled';
console.log('FeatureWithUI', FeatureWithUI);

const LazyComponent = lazy(() => import('./lazy-component'));

function LazyComponentContainer() {
  return (
    <Suspense fallback={<p>Loading...</p>}>
      <LazyComponent />
    </Suspense>
  );
}

const App = () => {
  const [showLazyComponent, setShowLazyComponent] = useState(false);

  return (
    <div>
      <p>Hello from React</p>
      <button onClick={() => setShowLazyComponent(!showLazyComponent)}>
        Toggle lazy component
      </button>
      <p>Conditional Feature: {Feature()}</p>
      <FeatureWithUI />
      {showLazyComponent ? <LazyComponentContainer /> : null}
    </div>
  );
};

// @ts-expect-error TS17004
ReactDOM.render(<App />, document.getElementById('container'));
