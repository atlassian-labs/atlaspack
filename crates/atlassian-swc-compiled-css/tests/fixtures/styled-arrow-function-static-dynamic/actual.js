import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._1reo15vq{overflow-x:hidden}";
const _1 = "._18m915vq{overflow-y:hidden}";
const _2 = "._1e0c1txw{display:flex}";
const _3 = "._1bah1h6o{justify-content:center}";
const _4 = "._4cvr1h6o{align-items:center}";
const _5 = "._1tke1mlo{min-height:var(--_rmnwww)}";
const gridSize = 8;
const Container = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    _5
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_rmnwww": ix(gridSize * (__cmplp.hideDropdownLabel ? 14 : 17), "px")
                },
                ref: __cmplr,
                className: ax([
                    "_1reo15vq _18m915vq _1e0c1txw _1bah1h6o _4cvr1h6o _1tke1mlo",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ({ hideDropdownLabel })=>jsx(Container, {
        hideDropdownLabel: hideDropdownLabel
    });
if (process.env.NODE_ENV !== "production") {
    Container.displayName = "Container";
}
