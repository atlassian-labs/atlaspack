import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React, { forwardRef } from 'react';
import { jsx } from "react/jsx-runtime";
const baseStyles = {
  boxSizing: 'border-box',
  appearance: 'none',
  border: 'none'
};

// Massive background color map (simplified from the original 100+ entries)
const backgroundColorMap = {
  'color.background.accent.lime.subtlest': "_bfhk1xib",
  'color.background.accent.lime.subtler': "_bfhkbkev",
  'color.background.accent.lime.subtle': "_bfhkhcps",
  'color.background.accent.red.subtlest': "_bfhk1xiw",
  'color.background.accent.red.subtler': "_bfhk1y41",
  'color.background.accent.red.subtle': "_bfhk1nbf",
  'color.background.accent.blue.subtlest': "_bfhk1xib",
  'color.background.accent.blue.subtler': "_bfhkbkev",
  'color.background.accent.blue.subtle': "_bfhkhcps",
  'color.background.neutral': "_bfhk1hxd",
  'color.background.neutral.hovered': "_bfhk19n3",
  'color.background.selected': "_bfhk1yjb",
  'elevation.surface': "_bfhku67f",
  'elevation.surface.raised': "_bfhku67f"
};

// CSS variables using unboundedCssMap
const CURRENT_SURFACE_CSS_VAR = '--ds-elevation-surface-current';
const setSurfaceTokenMap = {
  'elevation.surface': "_1q1lu67f",
  'elevation.surface.raised': "_1q1lu67f"
};

// Multiple padding maps
const paddingBlockStartMap = {
  'space.0': "_1q51idpf",
  'space.100': "_1q51ftgi",
  'space.200': "_1q517vkz",
  'space.300': "_1q511tcg"
};
const paddingInlineStartMap = {
  'space.0': "_bozgidpf",
  'space.100': "_bozgftgi",
  'space.200': "_bozg7vkz",
  'space.300': "_bozg1tcg"
};

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
  const {
    className: _spreadClass,
    ...safeHtmlAttributes
  } = htmlAttributes;
  const isSurfaceToken = bg => bg in setSurfaceTokenMap;
  return jsx(Component, {
    style: style,
    ref: ref,
    className: `
				${xcss || ''}
				${Object.keys(baseStyles).map(key => `${key}: ${baseStyles[key]}`).join('; ')}
				${backgroundColor ? backgroundColorMap[backgroundColor]() : ''}
				${backgroundColor && isSurfaceToken(backgroundColor) ? setSurfaceTokenMap[backgroundColor]() : ''}
				${paddingBlockStart ? paddingBlockStartMap[paddingBlockStart]() : ''}
				${paddingInlineStart ? paddingInlineStartMap[paddingInlineStart]() : ''}
			`.trim(),
    ...safeHtmlAttributes,
    "data-testid": testId,
    children: children
  });
});
Box.displayName = 'Box';
export default Box;
