import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React from 'react';
import { jsx } from "react/jsx-runtime";
const rowGapMap = {
  'space100': "_1p57ftgi",
  'space200': "_1p577vkz",
  'space300': "_1p571tcg"
};
const columnGapMap = {
  'space100': "_gy1pftgi",
  'space200': "_gy1p7vkz",
  'space300': "_gy1p1tcg"
};
const justifyContentMap = {
  start: "_1bah1y6m",
  center: "_1bah1h6o",
  end: "_1bahesu3"
};
const alignItemsMap = {
  start: "_4cvr1y6m",
  center: "_4cvr1h6o",
  end: "_4cvresu3"
};
const styles = {
  root: "_1e0c1txw _vchhusvi"
};

/**
 * __Flex__
 *
 * `Flex` is a primitive component that implements the CSS Flexbox API.
 */
const Flex = props => {
  const {
    as: Component = 'div',
    alignItems,
    justifyContent,
    gap,
    columnGap,
    rowGap,
    children
  } = props;
  return jsx(Component, {
    className: `
				${styles.root()} 
				${gap ? columnGapMap[gap]() : ''}
				${columnGap ? columnGapMap[columnGap]() : ''}
				${gap ? rowGapMap[gap]() : ''}
				${rowGap ? rowGapMap[rowGap]() : ''}
				${alignItems ? alignItemsMap[alignItems]() : ''}
				${justifyContent ? justifyContentMap[justifyContent]() : ''}
			`.trim(),
    children: children
  });
};
Flex.displayName = 'Flex';
export default Flex;
