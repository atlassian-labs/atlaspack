import { css } from '@compiled/react';

const styles = css({
  '& > [data-test]': {
    color: 'red',
  },
});

export const Component = () => <div css={styles} />;
