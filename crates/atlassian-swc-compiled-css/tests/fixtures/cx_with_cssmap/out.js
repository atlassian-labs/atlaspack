import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React from 'react';
import { cx } from '@compiled/react';
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "._19bvftgi{padding-left:8px}";
const _6 = "._n3tdftgi{padding-bottom:8px}";
const _5 = "._u5f3ftgi{padding-right:8px}";
const _4 = "._ca0qftgi{padding-top:8px}";
const _3 = "._1e0c1txw{display:flex}";
const _2 = "._4cvr1h6o{align-items:center}";
const _ = "._zulp1y44{gap:4px}";
const listStyles = {
  root: "_zulp1y44 _4cvr1h6o _1e0c1txw",
  popupContainer: "_ca0qftgi _u5f3ftgi _n3tdftgi _19bvftgi"
};
export function Component({
  children
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7]
    }), jsx("div", {
      xcss: cx(listStyles.root, listStyles.popupContainer),
      children: children
    })]
  });
}
