import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _9 = "._1e0c1ule{display:block}";
const _8 = "._2lx21bp4{flex-direction:column}";
const _7 = "._19bv7vkz{padding-left:1pc}";
const _6 = "._n3td7vkz{padding-bottom:1pc}";
const _5 = "._u5f37vkz{padding-right:1pc}";
const _4 = "._ca0q7vkz{padding-top:1pc}";
const _3 = "._1bah1yb4{justify-content:space-between}";
const _2 = "._4cvr1q9y{align-items:baseline}";
const _ = "._1e0c1txw{display:flex}";
const bodyStyles = null;
const imageStyles = null;
const defaultHeaderStyles = null;
const DefaultHeader = ({
  children
}) => jsxs(CC, {
  children: [jsx(CS, {
    children: [_, _2, _3]
  }), jsx("div", {
    className: ax(["_1e0c1txw _4cvr1q9y _1bah1yb4"]),
    children: children
  })]
});
function Component() {
  return jsxs("div", {
    children: [jsxs(CC, {
      children: [jsx(CS, {
        children: [_4, _5, _6, _7, _, _8]
      }), jsx("div", {
        className: ax(["_ca0q7vkz _u5f37vkz _n3td7vkz _19bv7vkz _1e0c1txw _2lx21bp4"]),
        children: "Body content"
      })]
    }), jsxs(CC, {
      children: [jsx(CS, {
        children: [_9]
      }), jsx("img", {
        src: "test.jpg",
        alt: "",
        className: ax(["_1e0c1ule"])
      })]
    }), jsx(DefaultHeader, {
      children: "Header"
    })]
  });
}
export default Component;
