import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._1doq1my7{color:teal}";
const _1 = "._12e81b2y{&:hover { color: black}";
const StyledDiv = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_1doq1my7 _12e81b2y",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(StyledDiv, {
        children: "Hover me"
    });
if (process.env.NODE_ENV !== "production") {
    StyledDiv.displayName = "StyledDiv";
}
