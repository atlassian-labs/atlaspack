import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._nd5ldkfm{grid-area:banner}";
const _1 = "._1reo15vq{overflow-x:hidden}";
const _2 = "._18m915vq{overflow-y:hidden}";
const _3 = "._4t3iutvi{height:var(--n_bnrM)}";
const _4 = "._152tidpf{inset-block-start:0}";
const _5 = "._kqsw1if8{position:sticky}";
const _6 = "._1pbyegat{z-index:4}";
const bannerMountedVar = '--n_bnrM';
const localSlotLayers = {
    banner: 4
};
const styles = {
    root: "_nd5ldkfm _1reo15vq _18m915vq _4t3iutvi _152tidpf _kqsw1if8 _1pbyegat"
};
export function Banner({ children, testId }) {
    return (jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _,
                    _1,
                    _2,
                    _3,
                    _4,
                    _5,
                    _6
                ]
            }),
            jsx("div", {
                "data-layout-slot": true,
                "data-testid": testId,
                className: ax([
                    "_nd5ldkfm _1reo15vq _18m915vq _4t3iutvi _152tidpf _kqsw1if8 _1pbyegat",
                    styles.root
                ]),
                children: children
            })
        ]
    }));
}
