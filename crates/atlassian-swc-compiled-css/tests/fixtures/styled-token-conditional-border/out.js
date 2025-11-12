import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._19itglyw{border:none}";
const _1 = "._19itzgxb{border:1px solid var(--ds-border,#091e4224)}";
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    __cmplp.isSummaryView ? "_19itglyw" : "_19itzgxb",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Wrapper, {
        isSummaryView: false
    });
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
