import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._189ee4h9{border-width:var(--ds-border-width,1px)}";
const _1 = "._1h6dmuej{border-color:var(--ds-border,#091e4224)}";
const _2 = "._19bvutpp{padding-left:var(--ds-space-150,9pt)}";
const _3 = "._1mspu2gc >span{margin-left:var(--ds-space-100,8px)}";
const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_189ee4h9 _1h6dmuej _19bvutpp _1mspu2gc",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Example = ()=>jsx(Component, {
        children: jsx("span", {
            children: "Child"
        })
    });
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
