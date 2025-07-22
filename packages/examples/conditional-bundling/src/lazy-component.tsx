import React from 'react';

// @ts-expect-error TS2304
const Button = importCond<
  typeof import('@atlaskit/button/new'),
  typeof import('@atlaskit/button')
>('my.feature.button', '@atlaskit/button', '@atlaskit/button/new');

// @ts-expect-error TS2304
const Feature = importCond<
  typeof import('./feature-enabled'),
  typeof import('./feature-disabled')
>('my.feature', './feature-enabled', './feature-disabled');

// @ts-expect-error TS2304
const FeatureLazy = importCond<
  typeof import('./feature-enabled-lazy'),
  typeof import('./feature-disabled-lazy')
>('my.feature.lazy', './feature-enabled-lazy', './feature-disabled-lazy');

export default function LazyComponent() {
  return (
    // @ts-expect-error TS17004
    <>
      {/*
       // @ts-expect-error TS17004 */}
      <p>
        {/*
         // @ts-expect-error TS17004 */}
        This is a lazy component. It has a button. <Button>Lazy button</Button>
      </p>
      {/*
       // @ts-expect-error TS17004 */}
      <p>Conditional Feature in lazy component: {Feature()}</p>
      {/*
       // @ts-expect-error TS17004 */}
      <p>Conditional Lazy Only Feature in lazy component: {FeatureLazy()}</p>
    </>
  );
}
