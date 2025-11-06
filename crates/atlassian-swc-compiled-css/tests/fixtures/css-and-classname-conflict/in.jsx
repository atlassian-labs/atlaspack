/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { cssMap, jsx } from '@compiled/react';

const styles = cssMap({
	root: {
		gridArea: 'aside',
		boxSizing: 'border-box',
		position: 'relative',
		'@media (min-width: 64rem)': {
			width: 'var(--aside-width)',
			justifySelf: 'end',
		},
	},
});

export function Component({ xcss, children }) {
	return (
		<aside
			css={styles.root}
			className={xcss}
		>
			{children}
		</aside>
	);
}