import React, {lazy, Suspense, useState} from 'react';
import ReactDOM from 'react-dom';
import RegularExport from './regular-import';
console.log(RegularExport);

const Feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');
const FeatureWithUI = importCond<
  typeof import('./feature-ui-enabled'),
  typeof import('./feature-ui-disabled')
>('feature.ui', './feature-ui-enabled', './feature-ui-disabled');

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

  console.log(Feature, FeatureWithUI);
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

ReactDOM.render(<App />, document.getElementById('container'));
