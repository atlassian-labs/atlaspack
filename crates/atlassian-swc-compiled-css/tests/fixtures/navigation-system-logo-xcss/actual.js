import React from 'react';
import { cx } from '@compiled/react';
import { token } from './tokens';
import { jsx } from "react/jsx-runtime";
const anchorStyles = {
    root: "_2rko12b0 _1e0c1txw _4cvr1h6o _4t3izwfg",
    newInteractionStates: "_irr3166n _1di64ot1"
};
const logoContainerStyles = {
    root: "_18zru2gc _1e0cglyw _p12fnklw _vchh18uv _10y41txw"
};
const LogoRenderer = ({ logoOrIcon })=>{
    return jsx("div", {
        children: logoOrIcon
    });
};
const Anchor = ({ children, xcss, ...props })=>{
    return jsx("a", {
        ...props,
        children: children
    });
};
export const CustomLogo = ({ href, logo, icon, onClick, label })=>{
    return jsx(Anchor, {
        "aria-label": label,
        href: href,
        xcss: cx(anchorStyles.root, anchorStyles.newInteractionStates),
        onClick: onClick,
        children: jsx("div", {
            css: [
                logoContainerStyles.root
            ],
            children: jsx(LogoRenderer, {
                logoOrIcon: logo
            })
        })
    });
};
