import { ax, ix, CC, CS } from "@compiled/react/runtime";
import * as _React from "react";
import React from 'react';
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1tkeuuw1{min-height:200px}";
const _1 = "._1tke68cl{min-height:90pt}";
const _2 = "._1reo15vq{overflow-x:hidden}";
const _3 = "._18m915vq{overflow-y:hidden}";
const _4 = "._1e0c1txw{display:flex}";
const _5 = "._1bah1h6o{justify-content:center}";
const _6 = "._4cvr1h6o{align-items:center}";
const Preloader = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _6
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1reo15vq _18m915vq _1e0c1txw _1bah1h6o _4cvr1h6o",
                    __cmplp.hideLabel ? "_1tkeuuw1" : "_1tke68cl",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ hideLabel })=>jsx(Preloader, {
        hideLabel: hideLabel,
        children: jsx("div", {
            children: "content"
        })
    });
if (process.env.NODE_ENV !== "production") {
    Preloader.displayName = "Preloader";
}
