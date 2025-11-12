/**
 * @jsxRuntime classic
 * @jsx jsx
 * @jsxFrag
 */
import { Fragment } from 'react';

import { cssMap, jsx } from '@compiled/react';

const mainElementStyles = cssMap({
	root: {
		gridArea: 'main',
		isolation: 'isolate',
		insetBlockStart: '48px',
		overflow: 'auto',
		'@media (min-width: 64rem)': {
			isolation: 'auto',
			height: 'calc(100vh - 48px)',
			position: 'sticky',
		},
	},
});

export function Main({
	children,
	xcss,
	testId,
}) {
	return (
		<Fragment>
			<div
				className={xcss}
				role="main"
				css={mainElementStyles.root}
				data-testid={testId}
			>
				{children}
			</div>
		</Fragment>
	);
}