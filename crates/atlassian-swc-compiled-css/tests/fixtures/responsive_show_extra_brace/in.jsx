/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx } from '@compiled/react';

const styles = {
	default: { display: 'none' },
	'above.xs': { '@media (min-width: 30rem)': { display: 'revert' } },
	'below.sm': { '@media not all and (min-width: 48rem)': { display: 'revert' } },
};

export const Show = ({ above, below, children }) => {
	return (
		<div
			css={[styles.default, above && styles[`above.${above}`], below && styles[`below.${below}`]]}
		>
			{children}
		</div>
	);
};