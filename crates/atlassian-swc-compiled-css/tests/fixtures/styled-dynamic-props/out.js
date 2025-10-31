import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._2rko1q3h{border-radius:9999px}";
const _1 = "._154i1yuh{top:var(--_r3iodj)}";
const _2 = "._1ltv9q3e{left:var(--_gzeubk)}";
const _3 = "._kqswstnw{position:absolute}";
const _4 = "._1bsb19bv{width:10px}";
const _5 = "._4t3i19bv{height:10px}";
const _6 = "._t9ecni0c{transform:translate(-5px,-5px)}";
const _7 = "._bfhk13q2{background-color:blue}";
const Dot = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2,
                    _3,
                    _4,
                    _5,
                    _6,
                    _7
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_r3iodj": ix(__cmplp.y, "px"),
                    "--_gzeubk": ix(__cmplp.x, "px")
                },
                ref: __cmplr,
                className: ax([
                    "_2rko1q3h _154i1yuh _1ltv9q3e _kqswstnw _1bsb19bv _4t3i19bv _t9ecni0c _bfhk13q2",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Dot, {
        x: 0,
        y: 0
    });
if (process.env.NODE_ENV !== "production") {
    Dot.displayName = "Dot";
}
