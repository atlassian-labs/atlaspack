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
	inner: {
		insetBlockStart: '48px',
		overflow: 'auto',
		height: '100%',
		'@media (min-width: 64rem)': {
			height: 'calc(100vh - 48px)',
			position: 'sticky',
		},
	},
});

export function Aside({
	children,
	xcss,
	label = 'Aside',
	testId,
}) {
	return (
		<aside
			aria-label={label}
			css={styles.root}
			data-testid={testId}
		>
			<div css={styles.inner}>
				{children}
			</div>
		</aside>
	);
}