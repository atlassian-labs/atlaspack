import Button from '@atlaskit/button/new';
import React from 'react';

export default function Component() {
  // @ts-expect-error TS17004
  return <Button>Hello button</Button>;
}
