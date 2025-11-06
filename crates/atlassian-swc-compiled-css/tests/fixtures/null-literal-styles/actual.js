import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1e0c1txw{display:flex}";
const _1 = "._4cvr1q9y{align-items:baseline}";
const _2 = "._1bah1yb4{justify-content:space-between}";
const bodyStyles = null;
const imageStyles = null;
const defaultHeaderStyles = null;
const DefaultHeader = ({ children })=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2
                ]
            }),
            jsx("div", {
                className: ax([
                    "_1e0c1txw _4cvr1q9y _1bah1yb4"
                ]),
                children: children
            })
        ]
    }));
function Component() {
    return jsxs("div", {
        children: [
            jsx("div", {
                css: bodyStyles,
                children: "Body content"
            }),
            jsx("img", {
                css: imageStyles,
                src: "test.jpg",
                alt: ""
            }),
            jsx(DefaultHeader, {
                children: "Header"
            })
        ]
    });
}
export default Component;
