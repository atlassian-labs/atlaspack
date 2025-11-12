import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1yt41l1e{padding:var(--_1b2fz9c) var(--_1398uva) var(--_1398uva) var(--_oxawwk)}";
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_1b2fz9c": ix("var(--ds-space-050, 4px)"),
                    "--_1398uva": ix("var(--ds-space-150, 12px)"),
                    "--_oxawwk": ix(__cmplp.padded ? "var(--ds-space-150, 12px)" : "var(--ds-space-0, 0px)")
                },
                ref: __cmplr,
                className: ax([
                    "_1yt41l1e",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Wrapper, {
        padded: true
    });
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
