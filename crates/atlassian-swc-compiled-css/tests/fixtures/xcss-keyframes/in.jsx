import React from 'react';
import { keyframes } from '@compiled/react';
import { xcss } from '@atlaskit/primitives';

const shimmer = keyframes({
  '0%': {
    fill: 'red',
  },
  '50%': {
    fill: 'blue',
  },
  '100%': {
    fill: 'red',
  },
});

const styles = xcss({
  width: '100%',
  animation: `${shimmer} 1s infinite`,
  background: 'red',
});

export const Component = () => <div xcss={styles} />;
