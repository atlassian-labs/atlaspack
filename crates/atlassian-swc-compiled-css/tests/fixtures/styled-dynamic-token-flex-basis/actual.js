import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._osiyu43s >*{flex-basis:var(--ds-space-200,1pc)}";
const _1 = "._osiy1r8g >*{flex-basis:var(--ds-space-500,40px)}";
const ListItem = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    __cmplp.isCompact ? "_osiyu43s" : "_osiy1r8g",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ isCompact })=>jsx(ListItem, {
        isCompact: isCompact,
        children: jsx("div", {
            children: "Content"
        })
    });
if (process.env.NODE_ENV !== "production") {
    ListItem.displayName = "ListItem";
}
