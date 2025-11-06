import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _11 = "._1i4qfg65{overflow-wrap:anywhere}";
const _10 = "._c71l53f4{max-height:75pt}";
const _1 = "._18m91wug{overflow-y:auto}";
const _0 = "._1reo1wug{overflow-x:auto}";
const _9 = "._11c81lyf{font:normal 14px/1.42857 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Oxygen,Ubuntu,Fira Sans,Droid Sans,Helvetica Neue,sans-serif}";
const _8 = "._syaz1jn0{color:var(--flag-icon-color)}";
const _7 = "._1o9zidpf{flex-shrink:0}";
const _6 = "._1bah1h6o{justify-content:center}";
const _5 = "._4cvr1h6o{align-items:center}";
const _4 = "._1tke1tcg{min-height:24px}";
const _3 = "._1ul91tcg{min-width:24px}";
const _2 = "._1e0c1txw{display:flex}";
const _ = "._1bsb1osq{width:100%}";
const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = null;
const iconWrapperStyles = null;
const flagWrapperStyles = null;
const Flag = ({
  description,
  testId
}) => {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_]
    }), jsxs("div", {
      role: "alert",
      "data-testid": testId,
      className: ax(["_1bsb1osq"]),
      children: [jsxs(CC, {
        children: [jsx(CS, {
          children: [_2, _3, _4, _5, _6, _7, _8]
        }), jsx("div", {
          className: ax(["_1e0c1txw _1ul91tcg _1tke1tcg _4cvr1h6o _1bah1h6o _1o9zidpf _syaz1jn0"]),
          children: "Icon"
        })]
      }), jsxs(CC, {
        children: [jsx(CS, {
          children: [_9, _0, _1, _10, _11]
        }), jsx("div", {
          className: ax(["_11c81lyf _1reo1wug _18m91wug _c71l53f4 _1i4qfg65"]),
          children: description
        })]
      })]
    })]
  });
};
export default Flag;
