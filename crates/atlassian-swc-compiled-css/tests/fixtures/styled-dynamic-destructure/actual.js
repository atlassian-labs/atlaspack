import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._v5641gda{transition:var(--_lbjvny)}";
const _1 = "._1bsb1rkg{width:var(--_1gljcou)}";
const _2 = "._1o9zidpf{flex-shrink:0}";
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
                    _2
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_1gljcou": ix(__cmplp.width),
                    "--_lbjvny": ix(`width ${__cmplp.duration}ms ease`)
                },
                ref: __cmplr,
                className: ax([
                    "_v5641gda _1bsb1rkg _1o9zidpf",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Wrapper, {
        width: "120px",
        duration: 200
    });
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
