import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx } from '@atlaskit/css';
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1bsb1osq{width:100%}";
const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = null;
const iconWrapperStyles = null;
const flagWrapperStyles = null;
const Flag = ({ description, testId })=>{
    return (jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsxs("div", {
                role: "alert",
                "data-testid": testId,
                className: ax([
                    "_1bsb1osq"
                ]),
                children: [
                    jsx("div", {
                        css: iconWrapperStyles,
                        children: "Icon"
                    }),
                    jsx("div", {
                        css: descriptionStyles,
                        children: description
                    })
                ]
            })
        ]
    }));
};
export default Flag;
