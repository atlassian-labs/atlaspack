import { css, keyframes } from '@compiled/react';

const shimmer = keyframes({
  '0%': {
    backgroundPosition: '-300px 0',
  },
  '100%': {
    backgroundPosition: '1000px 0',
  },
});

const styles = css({
  animationName: shimmer,
  animationDuration: '1s',
});

export const Component = () => <div css={styles} />;

