import { css } from '@compiled/react';
import { palette } from './palette';

const styles = css({
  color: palette.brand,
  ':hover': {
    backgroundColor: 'white',
  },
});

export const Component = () => (
  <div css={styles}>
    imported twice
  </div>
);
