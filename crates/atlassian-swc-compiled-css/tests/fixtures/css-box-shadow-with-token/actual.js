import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._16qs1s03{box-shadow:inset 0 -1px 0 0 var(--ds-border,#091e4224)}";
const token = (key, fallback)=>fallback;
const border = null;
const Component = ()=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsx("div", {
                className: ax([
                    "_16qs1s03"
                ]),
                children: jsx("span", {
                    children: "Content with border"
                })
            })
        ]
    }));
export default Component;
