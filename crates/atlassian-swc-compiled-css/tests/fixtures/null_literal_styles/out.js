import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = {};
const iconWrapperStyles = {};
const flagWrapperStyles = {};
const analyticsAttributes = {
  componentName: 'flag',
  packageName: 'test',
  packageVersion: '1.0.0'
};
function Flag() {
  return jsxs("div", {
    children: [jsx("span", {
      children: "Content"
    }), jsx("div", {
      children: "Test"
    })]
  });
}
export default Flag;
