import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._syazfrbu{color:var(--project-color-text)}";
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    !__cmplp.withSidebar && "_syazfrbu",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ withSidebar })=>jsx(Wrapper, {
        withSidebar: withSidebar
    });
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
