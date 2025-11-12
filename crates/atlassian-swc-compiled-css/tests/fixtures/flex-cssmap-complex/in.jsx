import React from 'react';
import { cssMap } from '@compiled/react';

const rowGapMap = cssMap({
	'space100': { rowGap: '8px' },
	'space200': { rowGap: '16px' },
	'space300': { rowGap: '24px' },
});

const columnGapMap = cssMap({
	'space100': { columnGap: '8px' },
	'space200': { columnGap: '16px' },
	'space300': { columnGap: '24px' },
});

const justifyContentMap = cssMap({
	start: { justifyContent: 'flex-start' },
	center: { justifyContent: 'center' },
	end: { justifyContent: 'flex-end' },
});

const alignItemsMap = cssMap({
	start: { alignItems: 'flex-start' },
	center: { alignItems: 'center' },
	end: { alignItems: 'flex-end' },
});

const styles = cssMap({
	root: {
		display: 'flex',
		boxSizing: 'border-box',
	},
});

/**
 * __Flex__
 *
 * `Flex` is a primitive component that implements the CSS Flexbox API.
 */
const Flex = (props) => {
	const {
		as: Component = 'div',
		alignItems,
		justifyContent,
		gap,
		columnGap,
		rowGap,
		children,
	} = props;
	
	return (
		<Component 
			className={`
				${styles.root()} 
				${gap ? columnGapMap[gap]() : ''}
				${columnGap ? columnGapMap[columnGap]() : ''}
				${gap ? rowGapMap[gap]() : ''}
				${rowGap ? rowGapMap[rowGap]() : ''}
				${alignItems ? alignItemsMap[alignItems]() : ''}
				${justifyContent ? justifyContentMap[justifyContent]() : ''}
			`.trim()}
		>
			{children}
		</Component>
	);
};

Flex.displayName = 'Flex';

export default Flex;