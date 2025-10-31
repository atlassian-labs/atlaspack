import * as React from 'react';
import { mq } from './breakpoints';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "@media (min-width:30em){._qhso1cnh{color:purple}}";
const styles = null;
export const Component = ()=>jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsx("div", {
                className: ax([
                    "_qhso1cnh"
                ]),
                children: "Hello"
            })
        ]
    });
