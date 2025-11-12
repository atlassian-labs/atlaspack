import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { forwardRef } from "react";
import { jsx, jsxs } from "react/jsx-runtime";
const _ = "._19pk1tcg{margin-top:24px}";
const _1 = "._19pk7vkz{margin-top:1pc}";
const titleStyles = {
    root: "_otyrftgi"
};
const FeatureCard = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_19pk1tcg",
                    __cmplp.className
                ])
            })
        ]
    });
});
const ButtonContainer = forwardRef(({ as: C = "div", style: __cmpls, ...__cmplp }, __cmplr)=>{
    if (__cmplp.innerRef) {
        throw new Error("Please use 'ref' instead of 'innerRef'.");
    }
    return jsxs(CC, {
        children: [
            jsx(CS, {
                children: [
                    _1
                ]
            }),
            jsx(C, {
                ...__cmplp,
                style: __cmpls,
                ref: __cmplr,
                className: ax([
                    "_19pk7vkz",
                    __cmplp.className
                ])
            })
        ]
    });
});
function FeatureCardView({ title, description }) {
    return jsxs(FeatureCard, {
        children: [
            jsx("div", {
                css: titleStyles.root,
                children: jsx("h3", {
                    children: title
                })
            }),
            jsx("div", {
                children: description
            }),
            jsx(ButtonContainer, {
                children: jsx("button", {
                    children: "Learn More"
                })
            })
        ]
    });
}
export default FeatureCardView;
if (process.env.NODE_ENV !== "production") {
    FeatureCard.displayName = "FeatureCard";
}
if (process.env.NODE_ENV !== "production") {
    ButtonContainer.displayName = "ButtonContainer";
}
