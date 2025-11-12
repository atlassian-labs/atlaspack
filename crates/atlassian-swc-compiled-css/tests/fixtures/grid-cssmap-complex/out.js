import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React from 'react';
import { jsx } from "react/jsx-runtime";
const rowGapMap = {
  'space100': "_1p57ftgi",
  'space200': "_1p577vkz",
  'space300': "_1p571tcg",
  'space400': "_1p57zwfg"
};
const columnGapMap = {
  'space100': "_gy1pftgi",
  'space200': "_gy1p7vkz",
  'space300': "_gy1p1tcg",
  'space400': "_gy1pzwfg"
};
const justifyContentMap = {
  start: "_1bah1y6m",
  center: "_1bah1h6o",
  end: "_1bahesu3",
  spaceBetween: "_1bah1yb4"
};
const alignContentMap = {
  start: "_ae4v1y6m",
  center: "_ae4v1h6o",
  end: "_ae4vesu3",
  spaceBetween: "_ae4v1yb4"
};
const alignItemsMap = {
  start: "_4cvr1y6m",
  center: "_4cvr1h6o",
  baseline: "_4cvr1q9y",
  end: "_4cvresu3"
};
const baseStyles = {
  root: "_1e0c11p5 _vchhusvi"
};
const gridAutoFlowMap = {
  row: "_wij2vrvc",
  column: "_wij21bp4",
  dense: "_wij218l3"
};

/**
 * __Grid__
 *
 * `Grid` is a primitive component that implements the CSS Grid API.
 */
const Grid = props => {
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
    autoFlow
  } = props;
  return jsx(Component, {
    id: id,
    className: `
				${baseStyles.root()} 
				${gap ? columnGapMap[gap]() : ''}
				${columnGap ? columnGapMap[columnGap]() : ''}
				${gap ? rowGapMap[gap]() : ''}
				${rowGap ? rowGapMap[rowGap]() : ''}
				${alignItems ? alignItemsMap[alignItems]() : ''}
				${alignContent ? alignContentMap[alignContent]() : ''}
				${justifyContent ? justifyContentMap[justifyContent]() : ''}
				${autoFlow ? gridAutoFlowMap[autoFlow]() : ''}
			`.trim(),
    children: children
  });
};
Grid.displayName = 'Grid';
export default Grid;
