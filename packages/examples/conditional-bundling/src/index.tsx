import React, {lazy, Suspense, useState} from 'react';
import ReactDOM from 'react-dom';
const RegularExport = require('./regular-import');
console.log(RegularExport);

const feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');
const featureWithUI = importCond<
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

  console.log(feature, featureWithUI);
  return (
    <div>
      <p>Hello from React</p>
      <button onClick={() => setShowLazyComponent(!showLazyComponent)}>
        Toggle lazy component
      </button>
      <p>Conditional Feature: {feature.Feature()}</p>
      <featureWithUI.Component />
      {showLazyComponent ? <LazyComponentContainer /> : null}
    </div>
  );
};

ReactDOM.render(<App />, document.getElementById('container'));
