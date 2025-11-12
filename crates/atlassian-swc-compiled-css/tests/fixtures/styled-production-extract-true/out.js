import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { jsx } from "react/jsx-runtime";
import { forwardRef } from "react";
export const Styled = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_1doq5scu",
            __cmplp.className
        ])
    });
});
if (process.env.NODE_ENV !== "production") {
    Styled.displayName = "Styled";
}
