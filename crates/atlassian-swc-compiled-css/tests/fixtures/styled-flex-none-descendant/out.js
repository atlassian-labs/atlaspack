import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from "react/jsx-runtime";
import { forwardRef } from "react";
const Item = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_16jlidpf _1o9zidpf _i0dl1wug _vdz4kb7n",
            __cmplp.className
        ])
    });
});
export const Component = ()=>jsx(Item, {
        children: jsx("span", {
            "data-target": "child",
            children: "Child"
        })
    });
if (process.env.NODE_ENV !== "production") {
    Item.displayName = "Item";
}
