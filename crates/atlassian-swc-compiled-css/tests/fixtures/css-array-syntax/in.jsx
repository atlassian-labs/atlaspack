import { css } from '@compiled/react';

const base = css({
  color: 'white',
});

const padding = css({
  padding: 16,
});

export const Component = () => (
  <div css={[base, padding]}>
    Hello
  </div>
);
