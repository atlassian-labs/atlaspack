import { cssMap } from '@compiled/react';

const styles = cssMap({
  primary: {
    color: 'salmon',
  },
  secondary: {
    color: 'goldenrod',
  },
});

export const Component = () => (
  <div>
    <span className={styles.primary()}>Primary</span>
    <span className={styles.secondary()}>Secondary</span>
  </div>
);
