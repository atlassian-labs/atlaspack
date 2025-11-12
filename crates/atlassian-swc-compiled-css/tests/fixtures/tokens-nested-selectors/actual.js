import * as React from 'react';
import { token } from '@atlaskit/tokens';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._2x4gze3t input{margin-top:var(--ds-space-0,0)}";
const _1 = "._12hv12x7 input{margin-right:var(--ds-space-075,6px)}";
const _2 = "._x5bdze3t input{margin-bottom:var(--ds-space-0,0)}";
const _3 = "._1rgf12x7 input{margin-left:var(--ds-space-075,6px)}";
const _4 = "._1t2q1gjq label{color:var(--ds-text-subtle,#44546f)}";
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _4
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_2x4gze3t _12hv12x7 _x5bdze3t _1rgf12x7 _1t2q1gjq",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Wrapper, {
        children: jsx("label", {
            children: jsx("input", {
                type: "checkbox"
            })
        })
    });
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
