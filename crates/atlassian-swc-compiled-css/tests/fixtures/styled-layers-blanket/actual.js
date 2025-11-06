import { ax, ix } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { layers } from '@atlassian/jira-common-styles/src/main.tsx';
import { jsx } from "react/jsx-runtime";
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_1pbybfng _kqsw1n9t",
            __cmplp.className
        ])
    });
});
export const Component = ()=>jsx(Wrapper, {});
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
