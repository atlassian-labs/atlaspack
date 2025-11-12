import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { Fragment } from 'react';
import { jsx, jsxs } from "react/jsx-runtime";
const _6 = "@media (min-width:64rem){._qwfh1wug{isolation:auto}._165tk7wh{height:calc(100vh - 3pc)}._13wn1if8{position:sticky}}";
const _5 = "._152tckbl{inset-block-start:3pc}";
const _4 = "._19121cl4{isolation:isolate}";
const _3 = "._18m91wug{overflow-y:auto}";
const _2 = "._1reo1wug{overflow-x:auto}";
const _ = "._nd5l1gzg{grid-area:main}";
const mainElementStyles = {
  root: "_nd5l1gzg _1reo1wug _18m91wug _19121cl4 _152tckbl _qwfh1wug _165tk7wh _13wn1if8"
};
export function Main({
  children,
  xcss,
  testId
}) {
  return jsx(Fragment, {
    children: jsxs(CC, {
      children: [jsx(CS, {
        children: [_, _2, _3, _4, _5, _6]
      }), jsx("div", {
        className: ax([mainElementStyles.root, xcss]),
        role: "main",
        "data-testid": testId,
        children: children
      })]
    })
  });
}
