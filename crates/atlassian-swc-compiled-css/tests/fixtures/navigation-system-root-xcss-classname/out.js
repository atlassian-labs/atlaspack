import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React from 'react';
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "@media (min-width:64rem){._12fkuz0r{grid-template-areas:\"banner banner banner\" \"top-bar top-bar top-bar\" \"side-nav main aside\"}._12qzrxre{grid-template-rows:auto auto 3fr}._1rqt70if{grid-template-columns:auto minmax(0,1fr) auto}}";
const _6 = "._1ciragmp >:not([data-layout-slot]){display:none!important}";
const _5 = "._2z0516ab{grid-template-rows:auto auto 1fr auto}";
const _4 = "._yv0ei47z{grid-template-columns:minmax(0,1fr)}";
const _3 = "._1lmcq9em{grid-template-areas:\"banner\" \"top-bar\" \"main\" \"aside\"}";
const _2 = "._1tke1kxc{min-height:100vh}";
const _ = "._1e0c11p5{display:grid}";
const styles = {
  root: "_1e0c11p5 _1tke1kxc _1lmcq9em _yv0ei47z _2z0516ab _1ciragmp _12fkuz0r _12qzrxre _1rqt70if"
};
export function Root({
  children,
  xcss,
  testId
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7]
    }), jsx("div", {
      className: ax([styles.root, xcss]),
      "data-testid": testId,
      children: children
    })]
  });
}
