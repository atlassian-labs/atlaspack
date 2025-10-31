import * as React from 'react';
import { palette } from './palette';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._syaze71q{color:#0a141ecc}";
const _1 = "._irr3u67f:hover{background-color:#fff}";
const styles = null;
export const Component = ()=>(jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1
                ]
            }),
            jsx("div", {
                className: ax([
                    "_syaze71q _irr3u67f"
                ]),
                children: "imported twice"
            })
        ]
    }));
