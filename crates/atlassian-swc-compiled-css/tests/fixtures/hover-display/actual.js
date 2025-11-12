import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._15y3r8wq:hover{--display-icon-before:var(--_b3nd5v)}";
const _1 = "._oyoh2fjc:hover{--display-drag-handle:var(--_11m4mys)}";
const _2 = "._tzy41kuy{opacity:.1}";
const tabStyles = null;
export const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_b3nd5v": ix(__cmplp.isDraggable ? 'none' : 'flex'),
                    "--_11m4mys": ix(__cmplp.isDraggable ? 'flex' : 'none')
                },
                ref: __cmplr,
                className: ax([
                    "_15y3r8wq _oyoh2fjc",
                    __cmplp.isDragging ? "_tzy41kuy" : "",
                    __cmplp.className
                ])
            })
        ]
    });
});
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
