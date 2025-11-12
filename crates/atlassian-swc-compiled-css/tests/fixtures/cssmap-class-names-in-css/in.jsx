import { cssMap, jsx } from '@compiled/react';

const asideVar = '--aside-var';
const panelSplitterResizingVar = '--n_asdRsz';
const contentInsetBlockStart = 'var(--content-inset-block-start)';
const contentHeightWhenFixed = 'var(--content-height-when-fixed)';

const styles = cssMap({
	root: {
		gridArea: 'aside',
		boxSizing: 'border-box',
		position: 'relative',
		'@media (min-width: 64rem)': {
			width: `var(${panelSplitterResizingVar}, var(${asideVar}))`,
			justifySelf: 'end',
		},
	},
	inner: {
		insetBlockStart: contentInsetBlockStart,
		overflow: 'auto',
		height: '100%',
		'@media (min-width: 64rem)': {
			height: contentHeightWhenFixed,
			position: 'sticky',
		},
	},
});

function AsideComponent({ children }) {
	return (
		<aside css={styles.root}>
			<div css={styles.inner}>
				{children}
			</div>
		</aside>
	);
}