import { css } from '@compiled/react';

const styles = css({
  margin: 5 + 5,
  padding: `${5 + 5}px`,
});

export const Component = () => <div css={styles}>Hello</div>;
