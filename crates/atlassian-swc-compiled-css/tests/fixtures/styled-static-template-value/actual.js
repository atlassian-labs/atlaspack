import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._1bsb7vkz{width:1pc}";
const _1 = "._1ul97vkz{min-width:1pc}";
const _2 = "._4t3i7vkz{height:1pc}";
const _3 = "._i0dlexct{flex-basis:16px}";
const GRID = 8;
const SIZE = GRID * 2;
const Icon = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _3
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1bsb7vkz _1ul97vkz _4t3i7vkz _i0dlexct",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Icon, {});
if (process.env.NODE_ENV !== "production") {
    Icon.displayName = "Icon";
}
