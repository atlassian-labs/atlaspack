import { css } from '@compiled/react';

const styles = css({
  '@property --my-color': {
    syntax: '"<color>"',
    inherits: false,
    initialValue: 'black',
  },
});

export const Component = () => <div css={styles}>Hello</div>;
