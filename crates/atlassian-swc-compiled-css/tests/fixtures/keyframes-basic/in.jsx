import { css, keyframes } from '@compiled/react';

const moveFade = keyframes({
	from: { opacity: 0 },
	to: { opacity: 1 },
});

<div css={css({ animationName: moveFade })} />;
