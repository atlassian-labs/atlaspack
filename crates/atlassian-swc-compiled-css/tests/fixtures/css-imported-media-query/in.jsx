import { css } from '@compiled/react';
import { mq } from './breakpoints';

const styles = css({
  [mq.small]: {
    color: 'purple',
  },
});

export const Component = () => <div css={styles}>Hello</div>;
