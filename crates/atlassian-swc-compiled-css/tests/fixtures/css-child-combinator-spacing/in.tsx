import { css } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const styles = css({
  display: 'flex',
  alignItems: 'center',
  '& > *': {
    marginLeft: token('space.100'),
  },
  '& > *:first-child': {
    marginLeft: 0,
  },
  '& > *:last-child': {
    marginRight: 0,
  },
});

export const Component = () => (
  <div css={styles}>
    <span />
    <span />
    <span />
  </div>
);
