import { forwardRef } from "react";
import { ax, ix } from "@compiled/react/runtime";
import React from "react";
import { jsx as _jsx } from "react/jsx-runtime";
export const Component = (props)=>{
    const Label = forwardRef(({ as: C = "h5", style: __cmpls, ...__cmplp }, __cmplr)=>{
        if (__cmplp.innerRef) {
            throw new Error("Please use 'ref' instead of 'innerRef'.");
        }
        return /*#__PURE__*/ _jsx(C, {
            ...__cmplp,
            style: {
                ...__cmpls,
                "--_16x5xn8": ix(props.isDisabled ? "var(--ds-text-disabled, #091E424F)" : "var(--ds-text, #172B4D)")
            },
            ref: __cmplr,
            className: ax([
                "_syaz1ukx",
                __cmplp.className
            ])
        });
    });
    if (process.env.NODE_ENV !== "production") {
        Label.displayName = "Label";
    }
    return /*#__PURE__*/ _jsx(Label, {
        children: "text"
    });
};
