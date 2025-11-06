import { ax, ix } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { jsx } from "react/jsx-runtime";
const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = null;
const iconWrapperStyles = null;
const flagWrapperStyles = null;
const analyticsAttributes = {
    componentName: 'flag',
    packageName: 'test',
    packageVersion: '1.0.0'
};
function Flag({ children }) {
    return jsx("div", {
        className: ax([]),
        children: jsx("span", {
            css: descriptionStyles,
            children: children
        })
    });
}
export default Flag;
