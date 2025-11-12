import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _1 = "._lcxvglyw{pointer-events:none}";
const _0 = "._tzy4105o{opacity:.5}";
const _9 = "._119sr3uz secondary{color:#000}";
const _8 = "._r5gp17nt secondary{background-color:gray}";
const _7 = "._vt6du67f primary{color:#fff}";
const _6 = "._s3uh13q2 primary{background-color:blue}";
const _5 = "._1e0c1txw{display:flex}";
const _4 = "._19bvftgi{padding-left:8px}";
const _3 = "._n3tdftgi{padding-bottom:8px}";
const _2 = "._u5f3ftgi{padding-right:8px}";
const _ = "._ca0qftgi{padding-top:8px}";
const baseStyles = null;
const variantStyles = null;
export const Component = ({
  variant,
  disabled,
  children
}) => {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7, _8, _9, _0, _1]
    }), jsx("div", {
      className: ax(["_ca0qftgi _u5f3ftgi _n3tdftgi _19bvftgi _1e0c1txw", variant && "_s3uh13q2 _vt6du67f _r5gp17nt _119sr3uz", disabled && "_tzy4105o _lcxvglyw"]),
      children: children
    })]
  });
};
