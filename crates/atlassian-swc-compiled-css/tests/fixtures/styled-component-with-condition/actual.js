import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { componentWithCondition } from '@atlassian/jira-feature-flagging-utils';
import { easeInOut } from '@atlaskit/motion';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._v5641lsu{transition:var(--_7nn7wk)}";
const _1 = "._1e0c1txw{display:flex}";
const _2 = "._njlp1rql{contain:layout}";
const _3 = "._1bsb1rkg{width:var(--_1gljcou)}";
const _4 = "._1reo15vq{overflow-x:hidden}";
const _5 = "._18m915vq{overflow-y:hidden}";
const _6 = "._1o9zidpf{flex-shrink:0}";
const OuterWrapperOld = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2,
                    _3
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_1gljcou": ix(__cmplp.width),
                    "--_7nn7wk": ix(`width ${__cmplp.duration}ms ${easeInOut}`)
                },
                ref: __cmplr,
                className: ax([
                    "_v5641lsu _1e0c1txw _njlp1rql _1bsb1rkg",
                    __cmplp.className
                ])
            })
        ]
    });
});
const OuterWrapperNew = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _4,
                    _5,
                    _1,
                    _2,
                    _3
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_1gljcou": ix(__cmplp.width),
                    "--_7nn7wk": ix(`width ${__cmplp.duration}ms ${easeInOut}`)
                },
                ref: __cmplr,
                className: ax([
                    "_v5641lsu _1reo15vq _18m915vq _1e0c1txw _njlp1rql _1bsb1rkg",
                    __cmplp.className
                ])
            })
        ]
    });
});
const OuterWrapper = componentWithCondition(()=>true, OuterWrapperNew, OuterWrapperOld);
const InnerWrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _6
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1o9zidpf",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ width, duration })=>jsx(OuterWrapper, {
        width: width,
        duration: duration,
        children: jsx(InnerWrapper, {})
    });
if (process.env.NODE_ENV !== "production") {
    OuterWrapperOld.displayName = "OuterWrapperOld";
}
if (process.env.NODE_ENV !== "production") {
    OuterWrapperNew.displayName = "OuterWrapperNew";
}
if (process.env.NODE_ENV !== "production") {
    InnerWrapper.displayName = "InnerWrapper";
}
