import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._nd5lns35{grid-area:aside}";
const _1 = "._vchhusvi{box-sizing:border-box}";
const _2 = "._kqswh2mm{position:relative}";
const _3 = "@media (min-width:64rem){._glte12kt{width:var(--n_asdRsz,var(--aside-var))}._ndwch9n0{justify-self:end}._165t8k1s{height:var(--content-height-when-fixed)}._13wn1if8{position:sticky}}";
const _4 = "._1reo1wug{overflow-x:auto}";
const _5 = "._18m91wug{overflow-y:auto}";
const _6 = "._152t1qck{inset-block-start:var(--content-inset-block-start)}";
const _7 = "._4t3i1osq{height:100%}";
const asideVar = '--aside-var';
const panelSplitterResizingVar = '--n_asdRsz';
const contentInsetBlockStart = 'var(--content-inset-block-start)';
const contentHeightWhenFixed = 'var(--content-height-when-fixed)';
const styles = {
    root: "_nd5lns35 _vchhusvi _kqswh2mm _glte12kt _ndwch9n0",
    inner: "_1reo1wug _18m91wug _152t1qck _4t3i1osq _165t8k1s _13wn1if8"
};
function AsideComponent({ children }) {
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
                    _6,
                    _7
                ]
            }),
            jsx("aside", {
                className: ax([
                    "_nd5lns35 _vchhusvi _kqswh2mm _glte12kt _ndwch9n0",
                    styles.root
                ]),
                children: jsx("div", {
                    css: styles.inner,
                    children: children
                })
            })
        ]
    }));
}
