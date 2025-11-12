import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._bfhk1paw{background-color:lime}";
const _1 = "._bfhk1i1c{background-color:cyan}";
const _2 = "._1ud41y44 .helper-class{padding-top:4px}";
const _3 = "._tozs1y44 .helper-class{padding-right:4px}";
const _4 = "._38gu1y44 .helper-class{padding-bottom:4px}";
const _5 = "._mn4j1y44 .helper-class{padding-left:4px}";
const _6 = "._10r21a6z .helper-class{color:tomato}";
const _7 = "._10jz105o #target-id:hover{opacity:.5}";
const _8 = "._12nkftgi #target-id{margin-top:8px}";
const _9 = "._syaz15td{color:#639}";
const selectors = {
    helper: 'helper-class',
    id: 'target-id'
};
const Component = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _3,
                    _4,
                    _5,
                    _6,
                    _7,
                    _8,
                    _9
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1ud41y44 _tozs1y44 _38gu1y44 _mn4j1y44 _10r21a6z _10jz105o _12nkftgi _syaz15td",
                    __cmplp.isActive ? "_bfhk1paw" : "_bfhk1i1c",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Styled = ()=>jsxs(Component, {
        isActive: true,
        children: [
            jsx("span", {
                className: "helper-class",
                children: "Helper"
            }),
            jsx("span", {
                id: "target-id",
                children: "Target"
            })
        ]
    });
if (process.env.NODE_ENV !== "production") {
    Component.displayName = "Component";
}
