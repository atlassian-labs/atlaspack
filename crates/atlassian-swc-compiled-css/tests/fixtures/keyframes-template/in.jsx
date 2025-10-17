import { css, keyframes } from '@compiled/react';

const from = { opacity: 0, color: 'red' };
const to = { opacity: 1, color: 'blue' };

const fade = keyframes`
  from { opacity: ${from.opacity}; color: ${from.color}; }
  to { opacity: ${to.opacity}; color: ${to.color}; }
`;

<div css={css({ animationName: fade })} />;
