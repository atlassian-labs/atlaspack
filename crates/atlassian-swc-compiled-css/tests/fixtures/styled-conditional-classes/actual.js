import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._80omtlke{cursor:pointer}";
const _1 = "._80om1wug{cursor:auto}";
const _2 = "._jomr5scu:focus, ._irr35scu:hover{background-color:red}";
const _3 = "._jomr18uv:focus, ._irr318uv:hover{background-color:initial}";
const _4 = "._nt751r31:focus, ._1dit1r31:hover{outline-color:currentColor}";
const _5 = "._49pcglyw:focus, ._ksodglyw:hover{outline-style:none}";
const _6 = "._1hvw1o36:focus, ._4hz81o36:hover{outline-width:medium}";
const _7 = "._1mizidpf >*{margin-top:0}";
const _8 = "._d4l71l7b >*{margin-right:3px}";
const _9 = "._8jx7idpf >*{margin-bottom:0}";
const _10 = "._1ko91l7b >*{margin-left:3px}";
const _11 = "._d4l7idpf >*{margin-right:0}";
const _12 = "._1ko9idpf >*{margin-left:0}";
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
                    _12
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1dit1r31 _nt751r31 _ksodglyw _49pcglyw _4hz81o36 _1hvw1o36",
                    __cmplp.isClickable ? "_irr35scu _jomr5scu" : "_irr318uv _jomr18uv",
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
