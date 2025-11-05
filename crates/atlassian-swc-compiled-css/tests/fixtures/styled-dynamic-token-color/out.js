import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx } from "react/jsx-runtime";
const ExpiryDateContainer = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: __cmpls,
        ref: __cmplr,
        className: ax([
            "_299mangw",
            __cmplp.dueInWeek ? "_syazrhrk" : "_syaz1kw7",
            __cmplp.className
        ])
    });
});
export const Component = ({ dueInWeek })=>jsx(ExpiryDateContainer, {
        dueInWeek: dueInWeek,
        children: "Content"
    });
if (process.env.NODE_ENV !== "production") {
    ExpiryDateContainer.displayName = "ExpiryDateContainer";
}
