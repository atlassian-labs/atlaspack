import { cssMap as cssMapUnbounded, jsx } from '@compiled/react';

const styles = cssMapUnbounded({
  container: {
    display: 'flex',
  },
  textBold: { 
    color: '#292A2E' 
  },
});

export const Component = () => (
  <div css={styles.container}>
    <span css={styles.textBold}>Content</span>
  </div>
);