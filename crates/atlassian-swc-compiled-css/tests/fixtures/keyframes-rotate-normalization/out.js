import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "@keyframes k1j8refv{0%{transform:rotate(0deg)}to{transform:rotate(1turn)}}";
const _1 = "._y44v32og{animation:k1j8refv 1.5s linear infinite}";
const spin = null;
export const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_y44v32og",
                    __cmplp.className
                ])
            })
        ]
    });
});
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
