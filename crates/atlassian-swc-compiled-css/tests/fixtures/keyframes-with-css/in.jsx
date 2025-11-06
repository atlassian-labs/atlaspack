import { css, keyframes } from '@compiled/react';

const fadeIn = keyframes({
  from: {
    opacity: 0,
    transform: 'translateY(20px)',
  },
  to: {
    opacity: 1,
    transform: 'translateY(0)',
  },
});

const animatedStyles = css({
  animation: `${fadeIn} 0.3s ease-in-out`,
  padding: '16px',
  backgroundColor: 'white',
  borderRadius: '4px',
});

export const Component = ({ children }) => {
  return <div css={animatedStyles}>{children}</div>;
};