import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from '@atlaskit/css';
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
        className: ax([]),
        children: [
            jsx("span", {
                css: descriptionStyles,
                children: "Content"
            }),
            jsx("div", {
                css: flagWrapperStyles,
                children: "Test"
            })
        ]
    });
}
export default Flag;
