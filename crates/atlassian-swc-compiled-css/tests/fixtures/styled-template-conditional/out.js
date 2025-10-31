import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from "react/jsx-runtime";
import { forwardRef } from "react";
const padding = '8px';
const large = 8;
const small = 4;
const Cell = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_ca0qftgi _n3tdftgi",
            __cmplp.first ? "_19bvftgi" : "_19bv1y44",
            __cmplp.last ? "_u5f3ftgi" : "_u5f31y44",
            __cmplp.className
        ])
    });
});
export const Component = ({ first, last })=>jsx(Cell, {
        first: first,
        last: last,
        children: "Content"
    });
if (process.env.NODE_ENV !== "production") {
    Cell.displayName = "Cell";
}
