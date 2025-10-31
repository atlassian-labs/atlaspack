import * as React from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { gridSize } from './constants';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._1ul9ys9h{min-width:5pc}";
const _1 = "._p12f1lit{max-width:10pc}";
const _2 = "._18u01wug{margin-left:auto}";
const _3 = "._2hwx1wug{margin-right:auto}";
const _4 = "._13l9h2mm td:first-child{position:relative}";
const _5 = "._p4liftgi td{padding-top:8px}";
const _6 = "._owip7vkz td{padding-right:1pc}";
const _7 = "._6aut1tcg td{padding-bottom:24px}";
const _8 = "._1pqwzwfg td{padding-left:2pc}";
const _9 = "._1bsb7vkz{width:1pc}";
const _10 = "._4t3i1tcg{height:24px}";
const _11 = "._1o9zidpf{flex-shrink:0}";
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
                    _5,
                    _6,
                    _7,
                    _8
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1ul9ys9h _p12f1lit _18u01wug _2hwx1wug _13l9h2mm _p4liftgi _owip7vkz _6aut1tcg _1pqwzwfg",
                    __cmplp.className
                ])
            })
        ]
    });
});
const Logo = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _9,
                    _10,
                    _11
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_1bsb7vkz _4t3i1tcg _1o9zidpf",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>_jsxs(Container, {
        children: [
            jsx(Logo, {}),
            jsx("table", {
                children: jsx("tbody", {
                    children: _jsxs("tr", {
                        children: [
                            jsx("td", {
                                children: "First"
                            }),
                            jsx("td", {
                                children: "Second"
                            })
                        ]
                    })
                })
            })
        ]
    });
if (process.env.NODE_ENV !== "production") {
    Container.displayName = "Container";
}
if (process.env.NODE_ENV !== "production") {
    Logo.displayName = "Logo";
}
