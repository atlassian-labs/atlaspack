import { css, keyframes } from '@compiled/react';

const kf = keyframes({
	'0%': { opacity: 0 },
	to: { opacity: 1 },
});

const a = css({ animationName: kf });
const b = css({ animationName: kf });

<>
	<div css={a} />
	<div css={b} />
</>;
