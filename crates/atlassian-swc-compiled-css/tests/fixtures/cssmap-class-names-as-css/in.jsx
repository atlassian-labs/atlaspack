/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import React from 'react';
import { cssMap, jsx } from '@compiled/react';

const styles = cssMap({
	root: {
		display: 'grid',
		minHeight: '100vh',
		gridTemplateAreas: `
            "banner"
            "top-bar"
            "main"
            "aside"
       `,
		gridTemplateColumns: 'minmax(0, 1fr)',
		gridTemplateRows: 'auto auto 1fr auto',
		'@media (min-width: 64rem)': {
			gridTemplateAreas: `
            "banner banner banner"
            "top-bar top-bar top-bar"
            "side-nav main aside"
       `,
			gridTemplateRows: 'auto auto 3fr',
			gridTemplateColumns: 'auto minmax(0,1fr) auto',
		},
		'@media (min-width: 90rem)': {
			gridTemplateAreas: `
                "banner banner banner banner"
                "top-bar top-bar top-bar top-bar"
                "side-nav main aside panel"
           `,
			gridTemplateRows: 'auto auto 3fr',
			gridTemplateColumns: 'auto minmax(0,1fr) auto auto',
		},
		'> :not([data-layout-slot])': {
			display: 'none !important',
		},
	},
});

export function Root({ children, xcss }) {
	return (
		<div css={styles.root}>
			{children}
		</div>
	);
}