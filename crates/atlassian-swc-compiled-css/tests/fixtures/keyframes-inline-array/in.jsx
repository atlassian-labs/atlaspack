import { css, keyframes } from '@compiled/react';

const Comp = () => {
	return (
		<div
			css={[
				{ color: 'red' },
				{ animationName: keyframes({ '0%': { opacity: 0 }, to: { opacity: 1 } }) },
			]}
		/>
	);
};
