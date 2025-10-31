import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._syaz1q9v{color:hotpink}";
const Base = ({ children })=>jsx("button", {
        children: children
    });
export const StyledButton = forwardRef(({ as: C = Base, style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_syaz1q9v",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(StyledButton, {
        children: "Click me"
    });
if (process.env.NODE_ENV !== "production") {
    StyledButton.displayName = "StyledButton";
}
