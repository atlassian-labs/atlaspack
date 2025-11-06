import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._80omtlke{cursor:pointer}";
const _1 = "._80om1wug{cursor:auto}";
const _2 = "._jomr5scu:focus{background-color:red}";
const _3 = "._irr35scu:hover{background-color:red}";
const _4 = "._jomr18uv:focus{background-color:initial}";
const _5 = "._irr318uv:hover{background-color:initial}";
const _6 = "._nt751r31:focus{outline-color:currentColor}";
const _7 = "._1dit1r31:hover{outline-color:currentColor}";
const _8 = "._49pcglyw:focus{outline-style:none}";
const _9 = "._ksodglyw:hover{outline-style:none}";
const _10 = "._1hvw1o36:focus{outline-width:medium}";
const _11 = "._4hz81o36:hover{outline-width:medium}";
const _12 = "._1mizidpf >*{margin-top:0}";
const _13 = "._d4l71l7b >*{margin-right:3px}";
const _14 = "._8jx7idpf >*{margin-bottom:0}";
const _15 = "._1ko91l7b >*{margin-left:3px}";
const _16 = "._d4l7idpf >*{margin-right:0}";
const _17 = "._1ko9idpf >*{margin-left:0}";
const Item = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _9,
                    _10,
                    _11,
                    _12,
                    _13,
                    _14,
                    _15,
                    _16,
                    _17
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_nt751r31 _1dit1r31 _49pcglyw _ksodglyw _1hvw1o36 _4hz81o36",
                    __cmplp.isClickable ? "_jomr5scu _irr35scu" : "_jomr18uv _irr318uv",
                    __cmplp.spaced ? "_1mizidpf _d4l71l7b _8jx7idpf _1ko91l7b" : "_1mizidpf _d4l7idpf _8jx7idpf _1ko9idpf",
                    __cmplp.isClickable ? "_80omtlke" : "_80om1wug",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ isClickable, spaced })=>jsx(Item, {
        isClickable: isClickable,
        spaced: spaced,
        children: "Content"
    });
if (process.env.NODE_ENV !== "production") {
    Item.displayName = "Item";
}
