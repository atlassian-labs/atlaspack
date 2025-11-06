import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1bsb4tsx{width:60pc}";
const Box = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1bsb4tsx",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Box, {
        children: "content"
    });
if (process.env.NODE_ENV !== "production") {
    Box.displayName = "Box";
}
