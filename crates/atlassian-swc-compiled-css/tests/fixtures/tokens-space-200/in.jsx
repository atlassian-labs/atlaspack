import { cssMap } from '@compiled/react';

const styles = cssMap({
  button: {
    paddingInline: 'var(--ds-space-200, 16px)',
  },
});

export const Component = () => <div className={styles.button()}>Content</div>;
