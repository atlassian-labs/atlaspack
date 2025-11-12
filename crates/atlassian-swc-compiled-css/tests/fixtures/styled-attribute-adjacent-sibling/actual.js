import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._jus61qr7 [data-field]+button{min-width:8pc}";
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
                    "_jus61qr7",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsxs(Container, {
        children: [
            jsx("span", {
                "data-field": true
            }),
            jsx("button", {
                type: "button",
                children: "Action"
            })
        ]
    });
if (process.env.NODE_ENV !== "production") {
    Container.displayName = "Container";
}
