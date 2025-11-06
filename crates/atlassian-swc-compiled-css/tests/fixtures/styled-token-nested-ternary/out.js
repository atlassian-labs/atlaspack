import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._bfhkmoow{background-color:var(--_qn7x4k)}";
export const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                style: {
                    ...__cmpls,
                    "--_qn7x4k": ix((()=>{
                        return __cmplp.isRowSelected ? "var(--ds-background-selected, #E9F2FF)" : __cmplp.formatRuleBackgroundColor ? __cmplp.formatRuleBackgroundColor : "var(--ds-background-neutral-subtle, #00000000)";
                    })())
                },
                ref: __cmplr,
                className: ax([
                    "_bfhkmoow",
                    __cmplp.className
                ])
            })
        ]
    });
});
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
