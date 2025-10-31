import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._19bvftgi{padding-left:8px}";
const _1 = "._19bv1y44{padding-left:4px}";
const _2 = "._u5f3ftgi{padding-right:8px}";
const _3 = "._u5f31y44{padding-right:4px}";
const _4 = "._ca0qidpf{padding-top:0}";
const _5 = "._u5f3idpf{padding-right:0}";
const _6 = "._n3tdidpf{padding-bottom:0}";
const _7 = "._19bvidpf{padding-left:0}";
const _8 = "._1e0c1o8l{display:inline-block}";
const gridSize = 8;
const HeadingCellWrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _7,
                    _8
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_ca0qidpf _u5f3idpf _n3tdidpf _19bvidpf _1e0c1o8l",
                    __cmplp.first ? "_19bvftgi" : "_19bv1y44",
                    __cmplp.last ? "_u5f3ftgi" : "_u5f31y44",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ first, last })=>jsx(HeadingCellWrapper, {
        first: first,
        last: last,
        children: "content"
    });
if (process.env.NODE_ENV !== "production") {
    HeadingCellWrapper.displayName = "HeadingCellWrapper";
}
