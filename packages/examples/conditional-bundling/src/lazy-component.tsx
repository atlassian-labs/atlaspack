import React from 'react';
const Button = importCond<
  typeof import('@atlaskit/button/new'),
  typeof import('@atlaskit/button')
>('my.feature.button', '@atlaskit/button', '@atlaskit/button/new');

const Feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');

const FeatureLazy = importCond<
  typeof import('./feature-enabled-lazy'),
  typeof import('./feature-disabled-lazy')
>('my.feature.lazy', './feature-enabled-lazy', './feature-disabled-lazy');

export default function LazyComponent() {
  return (
    <>
      <p>
        This is a lazy component. It has a button. <Button>Lazy button</Button>
      </p>
      <p>Conditional Feature in lazy component: {Feature()}</p>
      <p>Conditional Lazy Only Feature in lazy component: {FeatureLazy()}</p>
    </>
  );
}
