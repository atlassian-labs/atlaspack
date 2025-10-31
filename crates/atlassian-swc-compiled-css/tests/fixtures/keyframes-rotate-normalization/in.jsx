import { styled, keyframes } from '@compiled/react';

const spin = keyframes({
  '0%': {
    transform: 'rotate(0deg)',
  },
  '100%': {
    transform: 'rotate(360deg)',
  },
});

export const Component = styled.div({
  animation: `${spin} 1.5s linear infinite`,
});
