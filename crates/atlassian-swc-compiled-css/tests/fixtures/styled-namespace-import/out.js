import * as React from 'react';
import { colors } from './theme';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._bfhku67f{background-color:#fff}";
const _1 = "._syazalr3{color:#172b4d}";
const Box = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_bfhku67f _syazalr3",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Box, {
        children: "Hi"
    });
if (process.env.NODE_ENV !== "production") {
    Box.displayName = "Box";
}
