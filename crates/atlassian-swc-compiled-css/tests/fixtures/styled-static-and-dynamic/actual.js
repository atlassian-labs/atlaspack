import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._1itkikq0{background-image:var(--_wrma1b)}";
const _1 = "._1itkglyw{background-image:none}";
const _2 = "._i0dlexct{flex-basis:16px}";
const SIZE = 16;
const Icon = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_wrma1b": ix(__cmplp.url, ")", "url(")
                },
                ref: __cmplr,
                className: ax([
                    "_i0dlexct",
                    __cmplp.url ? "_1itkikq0" : "_1itkglyw",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ url })=>jsx(Icon, {
        url: url
    });
if (process.env.NODE_ENV !== "production") {
    Icon.displayName = "Icon";
}
