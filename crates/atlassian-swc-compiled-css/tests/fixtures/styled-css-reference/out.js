import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx } from "react/jsx-runtime";
const themedUnderline = {
    '&::after': {
        left: 0,
        content: "''"
    }
};
const tabStyles = null;
export const Tab = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_1q5t1r31 _1ohyglyw _1r9x1o36 _1e0c1txw _12k4idpf _t1p4b3bt",
            __cmplp.className
        ])
    });
});
if (process.env.NODE_ENV !== "production") {
    Tab.displayName = "Tab";
}
