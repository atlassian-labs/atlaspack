import { forwardRef } from 'react';
import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _3 = "._otyrftgi{margin-bottom:8px}";
const _2 = "._19pk7vkz{margin-top:1pc}";
const _ = "._19pk1tcg{margin-top:24px}";
const titleStyles = {
  root: "_otyrftgi"
};
const FeatureCard = forwardRef(({
  as: C = "div",
  style: __cmpls,
  ...__cmplp
}, __cmplr) => {
  if (__cmplp.innerRef) {
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  }
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_]
    }), jsx(C, {
      ...__cmplp,
      style: __cmpls,
      ref: __cmplr,
      className: ax(["_19pk1tcg", __cmplp.className])
    })]
  });
});
if (process.env.NODE_ENV !== 'production') {
  FeatureCard.displayName = 'FeatureCard';
}
const ButtonContainer = forwardRef(({
  as: C = "div",
  style: __cmpls,
  ...__cmplp
}, __cmplr) => {
  if (__cmplp.innerRef) {
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  }
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_2]
    }), jsx(C, {
      ...__cmplp,
      style: __cmpls,
      ref: __cmplr,
      className: ax(["_19pk7vkz", __cmplp.className])
    })]
  });
});
if (process.env.NODE_ENV !== 'production') {
  ButtonContainer.displayName = 'ButtonContainer';
}
function FeatureCardView({
  title,
  description
}) {
  return jsxs(FeatureCard, {
    children: [jsxs(CC, {
      children: [jsx(CS, {
        children: [_3]
      }), jsx("div", {
        className: ax([titleStyles.root]),
        children: jsx("h3", {
          children: title
        })
      })]
    }), jsx("div", {
      children: description
    }), jsx(ButtonContainer, {
      children: jsx("button", {
        children: "Learn More"
      })
    })]
  });
}
export default FeatureCardView;
