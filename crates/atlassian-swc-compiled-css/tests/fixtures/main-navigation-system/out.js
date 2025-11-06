import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { Fragment } from 'react';
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "._njlp1t7j{contain:paint}";
const _6 = "@media (min-width:64rem){._qwfh1wug{isolation:auto}._165teqxy{height:calc(100vh - var(--n_bnrM, 0px) - var(--n_tNvM, 0px))}._13wn1if8{position:sticky}}";
const _5 = "._152timx3{inset-block-start:calc(var(--n_bnrM, 0px) + var(--n_tNvM, 0px))}";
const _4 = "._19121cl4{isolation:isolate}";
const _3 = "._18m91wug{overflow-y:auto}";
const _2 = "._1reo1wug{overflow-x:auto}";
const _ = "._nd5l1gzg{grid-area:main}";
const contentHeightWhenFixed = `calc(100vh - var(--n_bnrM, 0px) - var(--n_tNvM, 0px))`;
const contentInsetBlockStart = `calc(var(--n_bnrM, 0px) + var(--n_tNvM, 0px))`;
const mainElementStyles = {
  root: "_nd5l1gzg _1reo1wug _18m91wug _19121cl4 _152timx3 _qwfh1wug _165teqxy _13wn1if8",
  containPaint: "_njlp1t7j"
};
export function Main({
  children,
  testId,
  id
}) {
  return jsx(Fragment, {
    children: jsxs(CC, {
      children: [jsx(CS, {
        children: [_, _2, _3, _4, _5, _6, _7]
      }), jsx("div", {
        id: id,
        "data-layout-slot": true,
        role: "main",
        "data-testid": testId,
        className: ax([mainElementStyles.root]),
        children: children
      })]
    })
  });
}
