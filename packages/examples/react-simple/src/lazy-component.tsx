import React from 'react';

import Feature from './feature-enabled';

import FeatureLazy from './feature-enabled-lazy';

export default function LazyComponent() {
  return (
    <>
      <p>
        This is a lazy component. It has a button. <button>Lazy button</button>
      </p>

      <p>Conditional Feature in lazy component: {Feature()}</p>

      <p>Conditional Lazy Only Feature in lazy component: {FeatureLazy()}</p>
    </>
  );
}
