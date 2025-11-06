import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "._1pbyegat{z-index:4}";
const _6 = "._kqsw1if8{position:sticky}";
const _5 = "._152tidpf{inset-block-start:0}";
const _4 = "._4t3iutvi{height:var(--n_bnrM)}";
const _3 = "._18m915vq{overflow-y:hidden}";
const _2 = "._1reo15vq{overflow-x:hidden}";
const _ = "._nd5ldkfm{grid-area:banner}";
const bannerMountedVar = '--n_bnrM';
const localSlotLayers = {
  banner: 4
};
const styles = {
  root: "_nd5ldkfm _1reo15vq _18m915vq _4t3iutvi _152tidpf _kqsw1if8 _1pbyegat"
};
export function Banner({
  children,
  testId
}) {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4, _5, _6, _7]
    }), jsx("div", {
      "data-layout-slot": true,
      "data-testid": testId,
      className: ax([styles.root]),
      children: children
    })]
  });
}
