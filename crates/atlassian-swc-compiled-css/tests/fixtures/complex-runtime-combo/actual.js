import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._ca0qftgi{padding-top:8px}";
const _1 = "._u5f3ftgi{padding-right:8px}";
const _2 = "._n3tdftgi{padding-bottom:8px}";
const _3 = "._19bvftgi{padding-left:8px}";
const _4 = "._1l201r31:focus ._1l201r31{outline-color:currentColor}";
const _5 = "._cwctglyw:focus ._cwctglyw{outline-style:none}";
const _6 = "._d7ut1o36:focus ._d7ut1o36{outline-width:medium}";
const _7 = "@keyframes k17e8rkr{\n  from { opacity: 0; }\n  to { opacity: 1; }\n}";
const _8 = "._y44v65d0{animation:k17e8rkr 2s linear}";
const _9 = '._aetr1vm8:after{content:"!"}';
const _10 = "._1wybdlk8{font-size:14px}";
const _11 = '@media (min-width:600px){._72bc18cn:hover{content:"hover"}._1qiwr3uz:hover{background-color:#000}}';
const _12 = "._syazxbvz{color:#1e90ff}";
const _13 = "._syaz14zx{color:crimson}";
const fade = null;
const toneMap = {
    primary: "_syazxbvz",
    danger: "_syaz14zx"
};
const baseStyles = null;
const Wrapper = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
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
                    "_ca0qftgi _u5f3ftgi _n3tdftgi _19bvftgi _1l201r31 _cwctglyw _d7ut1o36",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _7,
                    _8,
                    _9
                ]
            }),
            (jsxs(CC, {
                children: [
                    jsx(CS, {
                        children: [
                            _10,
                            _11,
                            _12,
                            _13
                        ]
                    }),
                    jsx(Wrapper, {
                        className: ax([
                            "_1wybdlk8 _72bc18cn _1qiwr3uz",
                            toneMap.primary
                        ]),
                        children: jsx("span", {
                            className: ax([
                                "_y44v65d0 _aetr1vm8"
                            ]),
                            children: "combo"
                        })
                    })
                ]
            }))
        ]
    }));
if (process.env.NODE_ENV !== "production") {
    Wrapper.displayName = "Wrapper";
}
