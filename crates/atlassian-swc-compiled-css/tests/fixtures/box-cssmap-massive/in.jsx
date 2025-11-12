import React, { forwardRef } from 'react';
import { jsx, cssMap as unboundedCssMap } from '@compiled/react';
import { cssMap } from '@compiled/react';

const baseStyles = {
	boxSizing: 'border-box',
	appearance: 'none',
	border: 'none',
};

// Massive background color map (simplified from the original 100+ entries)
const backgroundColorMap = cssMap({
	'color.background.accent.lime.subtlest': { backgroundColor: '#F0F8FF' },
	'color.background.accent.lime.subtler': { backgroundColor: '#E6F7FF' },
	'color.background.accent.lime.subtle': { backgroundColor: '#CCE7FF' },
	'color.background.accent.red.subtlest': { backgroundColor: '#FFF0F0' },
	'color.background.accent.red.subtler': { backgroundColor: '#FFE6E6' },
	'color.background.accent.red.subtle': { backgroundColor: '#FFCCCC' },
	'color.background.accent.blue.subtlest': { backgroundColor: '#F0F8FF' },
	'color.background.accent.blue.subtler': { backgroundColor: '#E6F7FF' },
	'color.background.accent.blue.subtle': { backgroundColor: '#CCE7FF' },
	'color.background.neutral': { backgroundColor: '#F4F5F7' },
	'color.background.neutral.hovered': { backgroundColor: '#EAECF0' },
	'color.background.selected': { backgroundColor: '#EBF5FF' },
	'elevation.surface': { backgroundColor: '#FFFFFF' },
	'elevation.surface.raised': { backgroundColor: '#FFFFFF' },
});

// CSS variables using unboundedCssMap
const CURRENT_SURFACE_CSS_VAR = '--ds-elevation-surface-current';
const setSurfaceTokenMap = unboundedCssMap({
	'elevation.surface': {
		[CURRENT_SURFACE_CSS_VAR]: '#FFFFFF',
	},
	'elevation.surface.raised': {
		[CURRENT_SURFACE_CSS_VAR]: '#FFFFFF',
	},
});

// Multiple padding maps
const paddingBlockStartMap = cssMap({
	'space.0': { paddingBlockStart: '0px' },
	'space.100': { paddingBlockStart: '8px' },
	'space.200': { paddingBlockStart: '16px' },
	'space.300': { paddingBlockStart: '24px' },
});

const paddingInlineStartMap = cssMap({
	'space.0': { paddingInlineStart: '0px' },
	'space.100': { paddingInlineStart: '8px' },
	'space.200': { paddingInlineStart: '16px' },
	'space.300': { paddingInlineStart: '24px' },
});

/**
 * __Box__
 *
 * A Box primitive component with massive cssMap configurations
 */
const Box = forwardRef((props, ref) => {
	const {
		as: Component = 'div',
		children,
		backgroundColor,
		paddingBlockStart,
		paddingInlineStart,
		style,
		testId,
		xcss,
		...htmlAttributes
	} = props;

	const { className: _spreadClass, ...safeHtmlAttributes } = htmlAttributes;
	
	const isSurfaceToken = (bg) => bg in setSurfaceTokenMap;

	return (
		<Component
			style={style}
			ref={ref}
			className={`
				${xcss || ''}
				${Object.keys(baseStyles).map(key => `${key}: ${baseStyles[key]}`).join('; ')}
				${backgroundColor ? backgroundColorMap[backgroundColor]() : ''}
				${backgroundColor && isSurfaceToken(backgroundColor) ? setSurfaceTokenMap[backgroundColor]() : ''}
				${paddingBlockStart ? paddingBlockStartMap[paddingBlockStart]() : ''}
				${paddingInlineStart ? paddingInlineStartMap[paddingInlineStart]() : ''}
			`.trim()}
			{...safeHtmlAttributes}
			data-testid={testId}
		>
			{children}
		</Component>
	);
});

Box.displayName = 'Box';

export default Box;