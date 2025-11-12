import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "._1wybdlk8{font-size:14px}";
const _6 = "._syaz5scu{color:red}";
const _5 = "._1e0c1txw{display:flex}";
const _4 = "._19bvftgi{padding-left:8px}";
const _3 = "._n3tdftgi{padding-bottom:8px}";
const _2 = "._u5f3ftgi{padding-right:8px}";
const _ = "._ca0qftgi{padding-top:8px}";
const styles = {
  container: "_ca0qftgi _u5f3ftgi _n3tdftgi _19bvftgi _1e0c1txw",
  text: "_syaz5scu _1wybdlk8"
};
export const Component = () => jsxs(CC, {
  children: [jsx(CS, {
    children: [_, _2, _3, _4, _5, _6, _7]
  }), jsx("div", {
    className: ax([styles.container]),
    children: jsxs(CC, {
      children: [jsx(CS, {
        children: [_, _2, _3, _4, _5, _6, _7]
      }), jsx("span", {
        className: ax([styles.text]),
        children: "Hello"
      })]
    })
  })]
});
