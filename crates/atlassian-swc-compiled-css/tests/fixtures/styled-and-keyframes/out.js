import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "@keyframes k1poetz8{0%{transform:scale(1)}50%{transform:scale(1.1)}to{transform:scale(1)}}";
const _1 = "._y44v1mmd{animation:k1poetz8 2s infinite}";
const pulse = null;
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
                    "_y44v1mmd",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(StyledDiv, {
        children: "Pulse"
    });
if (process.env.NODE_ENV !== "production") {
    StyledDiv.displayName = "StyledDiv";
}
