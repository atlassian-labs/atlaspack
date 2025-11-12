import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._syaz1lh4{color:var(--ds-text-disabled,#091e424f)}";
const _1 = "._syaz1fxt{color:var(--ds-text,#172b4d)}";
export const Component = (props)=>{
    const Label = forwardRef(({ as: C = "h5", style: __cmpls, ...__cmplp }, __cmplr)=>{
        if (__cmplp.innerRef) {
            throw new Error("Please use 'ref' instead of 'innerRef'.");
        }
        return jsxs(CC, {
            children: [
                jsx(CS, {
                    children: [
                        _,
                        _1
                    ]
                }),
                jsx(C, {
                    ...__cmplp,
                    style: __cmpls,
                    ref: __cmplr,
                    className: ax([
                        props.isDisabled ? "_syaz1lh4" : "_syaz1fxt",
                        __cmplp.className
                    ])
                })
            ]
        });
    });
    return jsx(Label, {
        children: "text"
    });
};
if (process.env.NODE_ENV !== "production") {
    Label.displayName = "Label";
}
