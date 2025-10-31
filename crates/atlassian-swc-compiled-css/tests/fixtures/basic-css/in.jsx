import { css } from '@compiled/react';

const styles = css({
  color: 'red',
  backgroundColor: 'transparent',
});

export const Component = () => <div css={styles}>Hello</div>;
