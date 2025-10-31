import * as React from 'react';
import { LAYOUT_OFFSET } from './layout-offset';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._4t3i1i8v{height:calc(100vh - var(--topNavigationHeight, 0px) - var(--bannerHeight, 0px))}";
const Container = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_4t3i1i8v",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Container, {
        children: "Content"
    });
if (process.env.NODE_ENV !== "production") {
    Container.displayName = "Container";
}
