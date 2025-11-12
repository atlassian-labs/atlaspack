import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = '._ect48cbs{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI","Roboto"}';
const fontFamily = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto'";
export const Component = forwardRef(({ as: C = "span", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_ect48cbs",
                    __cmplp.className
                ])
            })
        ]
    });
});
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
