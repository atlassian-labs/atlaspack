import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _4 = "@media (min-width:64rem){._glte25cg{width:var(--aside-width)}._ndwch9n0{justify-self:end}}";
const _3 = "._kqswh2mm{position:relative}";
const _2 = "._vchhusvi{box-sizing:border-box}";
const _ = "._nd5lns35{grid-area:aside}";
const styles = {
  root: "_nd5lns35 _vchhusvi _kqswh2mm _glte25cg _ndwch9n0"
};
export function Component({
  xcss,
  children
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4]
    }), jsx("aside", {
      className: ax([styles.root, xcss]),
      children: children
    })]
  });
}
