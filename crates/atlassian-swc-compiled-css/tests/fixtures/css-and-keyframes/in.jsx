import { css, keyframes } from '@compiled/react';

const fadeIn = keyframes({
  from: {
    opacity: 0,
  },
  to: {
    opacity: 1,
  },
});

const styles = css({
  animation: `${fadeIn} 1s ease`,
});

export const Component = () => <div css={styles}>Hello</div>;
