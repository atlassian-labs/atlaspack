import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { token } from '@atlaskit/tokens';
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1itk2i0w{background-image:linear-gradient(to right,var(--ds-background-neutral,#091e420f) 10%,var(--ds-background-neutral-subtle,#0000) 30%,var(--ds-background-neutral,#091e420f) 50%)}";
const Box = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_1itk2i0w",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(Box, {});
if (process.env.NODE_ENV !== "production") {
    Box.displayName = "Box";
}
