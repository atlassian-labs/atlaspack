import { css } from '@compiled/react';

export const styles = css({
  color: 'red',
  '&:hover': { color: 'blue' },
  '@media': {
    'screen and (min-width: 500px)': {
      color: 'green',
    },
  },
  content: '',
});
