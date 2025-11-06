import { css } from '@compiled/react';

const className = css({
  '--column-header-height': '48px',
});

export const Component = () => <div css={className}>custom property</div>;
