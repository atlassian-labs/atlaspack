import { css } from '@compiled/react';

const styles = css({
  MozAppearance: 'inherit',
  WebkitAppearance: 'none',
  msOverflowStyle: 'none',
  userSelect: 'none',
});

export const Component = () => <div css={styles} />;
