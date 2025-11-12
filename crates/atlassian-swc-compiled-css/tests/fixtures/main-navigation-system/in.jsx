/**
 * @jsxRuntime classic
 * @jsx jsx
 * @jsxFrag
 */
import { Fragment } from 'react';
import { cssMap, jsx } from '@compiled/react';

const contentHeightWhenFixed = `calc(100vh - var(--n_bnrM, 0px) - var(--n_tNvM, 0px))`;
const contentInsetBlockStart = `calc(var(--n_bnrM, 0px) + var(--n_tNvM, 0px))`;

const mainElementStyles = cssMap({
	root: {
		gridArea: 'main',
		isolation: 'isolate',
		insetBlockStart: contentInsetBlockStart,
		overflow: 'auto',
		'@media (min-width: 64rem)': {
			isolation: 'auto',
			height: contentHeightWhenFixed,
			position: 'sticky',
		},
	},
	containPaint: {
		contain: 'paint',
	},
});

export function Main({ children, testId, id }) {
	return (
		<Fragment>
			<div
				id={id}
				data-layout-slot
				role="main"
				css={mainElementStyles.root}
				data-testid={testId}
			>
				{children}
			</div>
		</Fragment>
	);
}