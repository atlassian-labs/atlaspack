import * as React from 'react';
import { colors } from '@atlaskit/theme';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from "react/jsx-runtime";
import { forwardRef } from "react";
const Box = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_bfhku67f _syazalr3",
            __cmplp.className
        ])
    });
});
export const Component = ()=>jsx(Box, {
        children: "Hi"
    });
if (process.env.NODE_ENV !== "production") {
    Box.displayName = "Box";
}
