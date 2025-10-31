import { cssMap } from '@atlaskit/css';

const styles = cssMap({
  base: {
    color: 'red',
    '&:hover': {
      color: 'blue',
    },
  },
});

export const Component = () => <div className={styles.base}>hello</div>;
