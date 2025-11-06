import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _1 = "@media not all and (min-width:110.5rem){._14wz19ly{display:revert}}";
const _0 = "@media not all and (min-width:90rem){._liwc19ly{display:revert}}";
const _9 = "@media not all and (min-width:64rem){._1mjb19ly{display:revert}}";
const _8 = "@media not all and (min-width:48rem){._suga19ly{display:revert}}";
const _7 = "@media not all and (min-width:30rem){._1m0a19ly{display:revert}}";
const _6 = "@media (min-width:110.5rem){._1uxv19ly{display:revert}}";
const _5 = "@media (min-width:90rem){._je3o19ly{display:revert}}";
const _4 = "@media (min-width:64rem){._dm2519ly{display:revert}}";
const _3 = "@media (min-width:48rem){._181n19ly{display:revert}}";
const _2 = "@media (min-width:30rem){._114b19ly{display:revert}}";
const _ = "._1e0cglyw{display:none}";
const styles = {
  default: "_1e0cglyw",
  'above.xs': "_114b19ly",
  'above.sm': "_181n19ly",
  'above.md': "_dm2519ly",
  'above.lg': "_je3o19ly",
  'above.xl': "_1uxv19ly",
  'below.xs': "_1m0a19ly",
  'below.sm': "_suga19ly",
  'below.md': "_1mjb19ly",
  'below.lg': "_liwc19ly",
  'below.xl': "_14wz19ly"
};
export const Show = ({
  above,
  below,
  children
}) => {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7, _8, _9, _0, _1]
    }), jsx("div", {
      className: ax([styles.default, above && styles[`above.${above}`], below && styles[`below.${below}`]]),
      children: children
    })]
  });
};
