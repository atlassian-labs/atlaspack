/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx, cssMap } from '@compiled/react';

const styles = cssMap({
	'above.xs': { '@media (min-width: 30rem)': { display: 'none' } },
	'above.sm': { '@media (min-width: 48rem)': { display: 'none' } },
	'above.md': { '@media (min-width: 64rem)': { display: 'none' } },
	'above.lg': { '@media (min-width: 90rem)': { display: 'none' } },
	'above.xl': { '@media (min-width: 110.5rem)': { display: 'none' } },
	'below.xs': { '@media not all and (min-width: 30rem)': { display: 'none' } },
	'below.sm': { '@media not all and (min-width: 48rem)': { display: 'none' } },
	'below.md': { '@media not all and (min-width: 64rem)': { display: 'none' } },
	'below.lg': { '@media not all and (min-width: 90rem)': { display: 'none' } },
	'below.xl': { '@media not all and (min-width: 110.5rem)': { display: 'none' } },
});

export const Hide = ({ above, below, children, as: AsElement = 'div', className }) => {
	return (
		<AsElement
			className={className}
			css={[above && styles[`above.${above}`], below && styles[`below.${below}`]]}
		>
			{children}
		</AsElement>
	);
};