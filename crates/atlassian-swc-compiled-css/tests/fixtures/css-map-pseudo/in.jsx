import { cssMap as unboundCssMap } from '@compiled/react';

const styles = unboundCssMap({
  base: {
    position: 'relative',
    margin: '8px',
    '&::before': {
      content: '',
      position: 'absolute',
      width: '100%',
      height: '100%',
    },
    '&:focus-within::before': {
      boxShadow: 'inset 0 0 0 2px blue',
    },
  },
});

export const Component = () => <div css={styles.base}>pseudo</div>;
