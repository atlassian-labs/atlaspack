import { css } from '@compiled/react';

const themedUnderline = {
  '&::after': {
    content: "''",
    position: 'absolute',
  },
};

const styles = css({
  "&[aria-expanded='true'], &[aria-current='page']": {
    display: 'flex',
    ...themedUnderline,
  },
});

export const Component = () => <div css={styles} />;

