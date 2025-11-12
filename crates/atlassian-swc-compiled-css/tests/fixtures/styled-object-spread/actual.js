import * as React from 'react';
import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { token } from '@atlaskit/tokens';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
import { forwardRef } from "react";
const _ = "._zulpu2gc{gap:var(--ds-space-100,8px)}";
const _1 = "._n7zl176a{border-bottom:var(--ds-space-025,2px) solid var(--ds-border,#091e4224)}";
const _2 = "._4cvr1h6o{align-items:center}";
const _3 = "._1e0c1txw{display:flex}";
const _4 = "._4t3i1jfw{height:var(--ds-space-500,40px)}";
const _5 = "._1bsb1osq{width:100%}";
const _6 = "._2x4gze3t input{margin-top:var(--ds-space-0,0)}";
const _7 = "._12hv12x7 input{margin-right:var(--ds-space-075,6px)}";
const _8 = "._x5bdze3t input{margin-bottom:var(--ds-space-0,0)}";
const _9 = "._1rgf12x7 input{margin-left:var(--ds-space-075,6px)}";
const _10 = "._dlecu2gc label{gap:var(--ds-space-100,8px)}";
const _11 = "._1eq11h6o label{align-items:center}";
const _12 = "._15pj1txw label{display:flex}";
const _13 = "._clfdze3t p{margin-top:var(--ds-space-0,0)}";
const _14 = "._11pqze3t p{margin-right:var(--ds-space-0,0)}";
const _15 = "._q0a5ze3t p{margin-bottom:var(--ds-space-0,0)}";
const _16 = "._c3k8ze3t p{margin-left:var(--ds-space-0,0)}";
const _17 = "._bfhkk4ro{background-color:var(--_zlgi7a)}";
const layoutStyles = {
    alignItems: 'center',
    display: 'flex',
    gap: `${token('space.100')}`
};
const checkboxStyles = {
    ...layoutStyles,
    borderBottom: `${token('space.025')} solid ${token('color.border')}`,
    height: `${token('space.500')}`,
    width: '100%'
};
const getBackgroundColor = (checked, disabled)=>{
    if (disabled) {
        return token('color.background.accent.gray.subtlest');
    }
    return checked ? token('color.background.accent.blue.subtlest') : 'transparent';
};
const ProjectCheckbox = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
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
                    _7,
                    _8,
                    _9,
                    _10,
                    _11,
                    _12,
                    _13,
                    _14,
                    _15,
                    _16,
                    _17
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: {
                    ...__cmpls,
                    "--_zlgi7a": ix(getBackgroundColor(__cmplp.checked, __cmplp.disabled))
                },
                ref: __cmplr,
                className: ax([
                    "_zulpu2gc _n7zl176a _4cvr1h6o _1e0c1txw _4t3i1jfw _1bsb1osq _2x4gze3t _12hv12x7 _x5bdze3t _1rgf12x7 _dlecu2gc _1eq11h6o _15pj1txw _clfdze3t _11pqze3t _q0a5ze3t _c3k8ze3t _bfhkk4ro",
                    __cmplp.className
                ])
            })
        ]
    });
});
export const Component = ()=>jsx(ProjectCheckbox, {
        checked: true,
        disabled: false,
        children: _jsxs("label", {
            children: [
                jsx("input", {
                    type: "checkbox"
                }),
                jsx("p", {
                    children: "Description"
                })
            ]
        })
    });
if (process.env.NODE_ENV !== "production") {
    ProjectCheckbox.displayName = "ProjectCheckbox";
}
