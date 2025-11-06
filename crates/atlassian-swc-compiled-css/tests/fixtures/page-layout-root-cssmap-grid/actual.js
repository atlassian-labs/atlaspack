import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1e0c11p5{display:grid}";
const _1 = "._1tke1kxc{min-height:100vh}";
const _2 = '._1lmcq9em{grid-template-areas:"banner" "top-bar" "main" "aside"}';
const _3 = "._yv0ei47z{grid-template-columns:minmax(0,1fr)}";
const _4 = "._2z0516ab{grid-template-rows:auto auto 1fr auto}";
const _5 = '@media (min-width:64rem){._12fkuz0r{grid-template-areas:"banner banner banner" "top-bar top-bar top-bar" "side-nav main aside"}._12qzrxre{grid-template-rows:auto auto 3fr}._1rqt70if{grid-template-columns:auto minmax(0,1fr) auto}}';
const _6 = '@media (min-width:90rem){._xkmgbaui{grid-template-areas:"banner banner banner banner" "top-bar top-bar top-bar top-bar" "side-nav main aside panel"}._jbc7rxre{grid-template-rows:auto auto 3fr}._tyve1jg8{grid-template-columns:auto minmax(0,1fr) auto auto}}';
const styles = {
    root: "_1e0c11p5 _1tke1kxc _1lmcq9em _yv0ei47z _2z0516ab _12fkuz0r _12qzrxre _1rqt70if _xkmgbaui _jbc7rxre _tyve1jg8"
};
export function Root({ children }) {
    return (jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2,
                    _3,
                    _4,
                    _5,
                    _6
                ]
            }),
            jsx("div", {
                className: ax([
                    "_1e0c11p5 _1tke1kxc _1lmcq9em _yv0ei47z _2z0516ab _12fkuz0r _12qzrxre _1rqt70if _xkmgbaui _jbc7rxre _tyve1jg8",
                    styles.root
                ]),
                children: children
            })
        ]
    }));
}
