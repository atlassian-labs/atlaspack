import React from 'react';
import { cssMap } from '@compiled/react';

const rowGapMap = cssMap({
	'space100': { rowGap: '8px' },
	'space200': { rowGap: '16px' },
	'space300': { rowGap: '24px' },
	'space400': { rowGap: '32px' },
});

const columnGapMap = cssMap({
	'space100': { columnGap: '8px' },
	'space200': { columnGap: '16px' },
	'space300': { columnGap: '24px' },
	'space400': { columnGap: '32px' },
});

const justifyContentMap = cssMap({
	start: { justifyContent: 'flex-start' },
	center: { justifyContent: 'center' },
	end: { justifyContent: 'flex-end' },
	spaceBetween: { justifyContent: 'space-between' },
});

const alignContentMap = cssMap({
	start: { alignContent: 'flex-start' },
	center: { alignContent: 'center' },
	end: { alignContent: 'flex-end' },
	spaceBetween: { alignContent: 'space-between' },
});

const alignItemsMap = cssMap({
	start: { alignItems: 'flex-start' },
	center: { alignItems: 'center' },
	baseline: { alignItems: 'baseline' },
	end: { alignItems: 'flex-end' },
});

const baseStyles = cssMap({
	root: {
		display: 'grid',
		boxSizing: 'border-box',
	},
});

const gridAutoFlowMap = cssMap({
	row: { gridAutoFlow: 'row' },
	column: { gridAutoFlow: 'column' },
	dense: { gridAutoFlow: 'dense' },
});

/**
 * __Grid__
 *
 * `Grid` is a primitive component that implements the CSS Grid API.
 */
const Grid = (props) => {
	const {
		as: Component = 'div',
		alignItems,
		alignContent,
		justifyContent,
		gap,
		columnGap,
		rowGap,
		children,
		id,
		autoFlow,
	} = props;

	return (
		<Component
			id={id}
			className={`
				${baseStyles.root()} 
				${gap ? columnGapMap[gap]() : ''}
				${columnGap ? columnGapMap[columnGap]() : ''}
				${gap ? rowGapMap[gap]() : ''}
				${rowGap ? rowGapMap[rowGap]() : ''}
				${alignItems ? alignItemsMap[alignItems]() : ''}
				${alignContent ? alignContentMap[alignContent]() : ''}
				${justifyContent ? justifyContentMap[justifyContent]() : ''}
				${autoFlow ? gridAutoFlowMap[autoFlow]() : ''}
			`.trim()}
		>
			{children}
		</Component>
	);
};

Grid.displayName = 'Grid';

export default Grid;