import { cssMap as cssMapUnbounded, jsx } from '@compiled/react';

const styles = cssMapUnbounded({
  container: {
    display: 'flex',
    padding: '8px',
  },
  text: {
    color: 'red',
    fontSize: '14px',
  },
});

export const Component = () => (
  <div css={styles.container}>
    <span css={styles.text}>Hello</span>
  </div>
);