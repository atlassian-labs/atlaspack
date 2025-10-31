import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = '._1kt9b3bt:before{content:""}';
const _1 = '._aetr1e8g:after{content:"hello"}';
const _2 = "._1q7w1isi span:before{content:attr(data-label)}";
const styles = null;
export const Component = ()=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2
                ]
            }),
            jsx("div", {
                className: ax([
                    "_1kt9b3bt _aetr1e8g _1q7w1isi"
                ]),
                children: jsx("span", {
                    "data-label": "test"
                })
            })
        ]
    }));
