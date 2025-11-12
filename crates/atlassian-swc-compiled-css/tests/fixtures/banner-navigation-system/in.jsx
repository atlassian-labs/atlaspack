/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { cssMap, jsx } from '@compiled/react';

const bannerMountedVar = '--n_bnrM';
const localSlotLayers = {
	banner: 4,
};

const styles = cssMap({
	root: {
		gridArea: 'banner',
		height: `var(${bannerMountedVar})`,
		insetBlockStart: 0,
		position: 'sticky',
		zIndex: localSlotLayers.banner,
		overflow: 'hidden',
	},
});

export function Banner({ children, testId }) {
	return (
		<div data-layout-slot css={styles.root} data-testid={testId}>
			{children}
		</div>
	);
}