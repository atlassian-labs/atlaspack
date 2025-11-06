/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import React from 'react';
import { cx, jsx } from '@compiled/react';
import { cssMap } from '@atlaskit/css';

const listStyles = cssMap({
	root: {
		alignItems: 'center',
		gap: '4px',
		display: 'flex',
	},
	popupContainer: {
		padding: '8px',
	},
});

export function Component({ children }) {
	return (
		<div xcss={cx(listStyles.root, listStyles.popupContainer)}>
			{children}
		</div>
	);
}