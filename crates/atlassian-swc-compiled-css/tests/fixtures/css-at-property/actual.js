import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = '@property --my-color{syntax:"<color>";inherits:false;initial-value:black}';
const styles = null;
export const Component = ()=>jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsx("div", {
                className: ax([]),
                children: "Hello"
            })
        ]
    });
