import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _9 = "@media (min-width:64rem){._165tk7wh{height:calc(100vh - 3pc)}._13wn1if8{position:sticky}}";
const _8 = "._4t3i1osq{height:100%}";
const _7 = "._152tckbl{inset-block-start:3pc}";
const _6 = "._18m91wug{overflow-y:auto}";
const _5 = "._1reo1wug{overflow-x:auto}";
const _4 = "@media (min-width:64rem){._glte25cg{width:var(--aside-width)}._ndwch9n0{justify-self:end}}";
const _3 = "._kqswh2mm{position:relative}";
const _2 = "._vchhusvi{box-sizing:border-box}";
const _ = "._nd5lns35{grid-area:aside}";
const styles = {
  root: "_nd5lns35 _vchhusvi _kqswh2mm _glte25cg _ndwch9n0",
  inner: "_1reo1wug _18m91wug _152tckbl _4t3i1osq _165tk7wh _13wn1if8"
};
export function Aside({
  children,
  xcss,
  label = 'Aside',
  testId
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7, _8, _9]
    }), jsx("aside", {
      "aria-label": label,
      "data-testid": testId,
      className: ax([styles.root]),
      children: jsxs(CC, {
        children: [jsx(CS, {
          children: [_, _2, _3, _4, _5, _6, _7, _8, _9]
        }), jsx("div", {
          className: ax([styles.inner]),
          children: children
        })]
      })
    })]
  });
}
