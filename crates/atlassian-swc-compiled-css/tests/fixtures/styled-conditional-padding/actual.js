import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._1yt418x6{padding:4px 8px 8px var(--_pns0k)}";
const PaddingWrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "--_pns0k": ix(__cmplp.isSummaryView ? '0px' : '12px')
                },
                ref: __cmplr,
                className: ax([
                    "_1yt418x6",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(PaddingWrapper, {
        isSummaryView: false,
        children: "Content"
    });
if (process.env.NODE_ENV !== "production") {
    PaddingWrapper.displayName = "PaddingWrapper";
}
