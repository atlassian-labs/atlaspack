import Button from '@atlaskit/button/new';
import React from 'react';

export default function Component() {
// @ts-expect-error - TS17004 - Cannot use JSX unless the '--jsx' flag is provided.
  return <Button>Hello button</Button>;
}
