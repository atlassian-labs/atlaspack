import { css } from '@compiled/react';

const styles = css({
  color: 'rebeccapurple',
  backgroundColor: 'whitesmoke',
});

export const Component = () => (
  <div className="legacy" css={styles}>
    hello world
  </div>
);
