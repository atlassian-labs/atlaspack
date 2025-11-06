import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { SimpleTag as Tag } from '@atlaskit/tag';
import React from 'react';
import { jsx, jsxs } from "react/jsx-runtime";
const _0 = "._1p1dangw{text-transform:uppercase}";
const _9 = "._k48p8n31{font-weight:bold}";
const _8 = "._1wyb1crf{font-size:9pt}";
const _7 = "._1e0c1o8l{display:inline-block}";
const _6 = "._19it14it{border:1px solid #ccc}";
const _5 = "._2rko1l7b{border-radius:3px}";
const _4 = "._19bvftgi{padding-left:8px}";
const _3 = "._n3td1y44{padding-bottom:4px}";
const _2 = "._u5f3ftgi{padding-right:8px}";
const _ = "._ca0q1y44{padding-top:4px}";
// eslint-disable-next-line @atlaskit/design-system/no-emotion-primitives – to be migrated to @atlaskit/primitives/compiled

const styles = {
  tag: "_ca0q1y44 _u5f3ftgi _n3td1y44 _19bvftgi _2rko1l7b _19it14it _1e0c1o8l _1wyb1crf _k48p8n31 _1p1dangw"
};
export function TagComponent({
  children,
  color
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7, _8, _9, _0]
    }), jsx(Tag, {
      color: color,
      className: ax([styles.tag]),
      children: children
    })]
  });
}
