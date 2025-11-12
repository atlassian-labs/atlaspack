import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1e0c1txw{display:flex}";
const _1 = "._2lx21bp4{flex-direction:column}";
const bodyStyles = null;
const imageStyles = null;
const defaultHeaderStyles = null;
const Component = ()=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1
                ]
            }),
            jsxs("div", {
                className: ax([
                    "_1e0c1txw _2lx21bp4"
                ]),
                children: [
                    jsx("img", {
                        css: imageStyles,
                        src: "test.jpg",
                        alt: ""
                    }),
                    jsx("div", {
                        css: defaultHeaderStyles,
                        children: "Header"
                    })
                ]
            })
        ]
    }));
export default Component;
