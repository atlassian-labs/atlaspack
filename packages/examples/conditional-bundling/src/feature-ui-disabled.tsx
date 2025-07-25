import React from 'react';

export default function Component() {
  // @ts-expect-error TS17004
  return <button>No fancy</button>;
}
