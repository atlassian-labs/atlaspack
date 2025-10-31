import { css } from '@compiled/react';

const styles = css({
  color: 'navy',
  '&:hover': {
    color: 'white',
  },
  '& > span': {
    textDecoration: 'underline',
  },
  '@media (min-width: 50em)': {
    '&:focus': {
      outline: 'none',
    },
  },
});

export const Component = () => (
  <div css={styles}>
    <span>Nested</span>
  </div>
);
