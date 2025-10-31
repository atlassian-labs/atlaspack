import { css } from '@compiled/react';

const styles = css({
  '&::before': {
    content: '',
  },
  '&::after': {
    content: '"hello"',
  },
  '& span::before': {
    content: 'attr(data-label)',
  },
});

export const Component = () => (
  <div css={styles}>
    <span data-label="test" />
  </div>
);
