import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx } from "react/jsx-runtime";
const DotStart = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: {
            ...__cmpls,
            "--_bajtf7": ix(`${__cmplp.y}px`),
            "--_172sugn": ix(`${__cmplp.x}px`)
        },
        ref: __cmplr,
        className: ax([
            "_2rko1rr0 _kqswstnw _154i1wux _1ltv1j37 _1bsb19bv _4t3i19bv _t9ecni0c _bfhk4v9p",
            __cmplp.className
        ])
    });
});
const DotEnd = forwardRef(({ as: C = DotStart, style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_bfhk1rtt",
            __cmplp.className
        ])
    });
});
export const Example = ()=>jsx(DotEnd, {
        x: 10,
        y: 20
    });
if (process.env.NODE_ENV !== "production") {
    DotStart.displayName = "DotStart";
}
if (process.env.NODE_ENV !== "production") {
    DotEnd.displayName = "DotEnd";
}
