import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { token } from '@atlaskit/tokens';
import { jsx } from "react/jsx-runtime";
const SkeletonRow = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsx(C, {
        ...__cmplp,
        style: {
            ...__cmpls,
            "--_1neqek2": ix(`${__cmplp.height}px`),
            "--_6f8077": ix(`${__cmplp.width}px`)
        },
        ref: __cmplr,
        className: ax([
            "_2rko12b0 _1itk1wva _12vemgnk _4t3i2nrh _1bsb1hdq",
            __cmplp.className
        ])
    });
});
export const Component = ()=>jsx(SkeletonRow, {
        height: 40,
        width: 200
    });
if (process.env.NODE_ENV !== "production") {
    SkeletonRow.displayName = "SkeletonRow";
}
