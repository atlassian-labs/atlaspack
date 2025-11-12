import { forwardRef } from "react";
import * as React from "react";
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
const _ = "._jus61qr7 [data-field]+button{min-width:8pc}";
const Container = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return /*#__PURE__*/ _jsxs(CC, {
        children: [
            /*#__PURE__*/ _jsx(CS, {
                children: [
                    _
                ]
            }),
            /*#__PURE__*/ _jsx(C, {
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
if (process.env.NODE_ENV !== "production") {
    Container.displayName = "Container";
}
export const Component = ()=>/*#__PURE__*/ _jsxs(Container, {
    children: [
        /*#__PURE__*/ _jsx("span", {
            "data-field": true
        }),
        /*#__PURE__*/ _jsx("button", {
            type: "button",
            children: "Action"
        })
    ]
});
