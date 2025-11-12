import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React from 'react';
import { cx } from '@atlaskit/css';
import { jsx, jsxs } from "react/jsx-runtime";
const _11 = "._4t3izwfg{height:2pc}";
const _10 = "._1bsbzwfg{width:2pc}";
const _1 = "._4t3i1tcg{height:24px}";
const _0 = "._1bsb1tcg{width:24px}";
const _9 = "._4t3i7vkz{height:1pc}";
const _8 = "._1bsb7vkz{width:1pc}";
const _7 = "._1bah1h6o{justify-content:center}";
const _6 = "._4cvr1h6o{align-items:center}";
const _5 = "._1e0c1txw{display:flex}";
const _4 = "._189et94y{border-width:1px}";
const _3 = "._1h6d14ap{border-color:#ccc}";
const _2 = "._1dqonqa1{border-style:solid}";
const _ = "._2rko1y44{border-radius:4px}";
const styles = {
  goalIcon: "_2rko1y44 _1dqonqa1 _1h6d14ap _189et94y _1e0c1txw _4cvr1h6o _1bah1h6o",
  size16: "_1bsb7vkz _4t3i7vkz",
  size24: "_1bsb1tcg _4t3i1tcg",
  size32: "_1bsbzwfg _4t3izwfg"
};
export function GoalIcon({
  size = '24'
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7, _8, _9, _0, _1, _10, _11]
    }), jsx("div", {
      xcss: cx(styles.goalIcon, styles[`size${size}`]),
      children: "Content"
    })]
  });
}
