import React from 'react';
import Button from '@atlaskit/button';

export default function LazyComponent() {
  return (
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
    <p>
{ /* @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided. */}
      This is a lazy component. It has a button. <Button>Lazy button</Button>
    </p>
  );
}
