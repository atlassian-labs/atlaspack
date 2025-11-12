import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { token } from './tokens';
import { jsx, jsxs } from "react/jsx-runtime";
const _7 = "@media (prefers-reduced-motion:reduce){._1bumglyw{animation:none}._sedtglyw{transition:none}}";
const _6 = "._16qs1e2z{box-shadow:var(--_ztiv7h)}";
const _5 = "._1pglmcjr{animation-timing-function:cubic-bezier(.55,.055,.675,.19)}";
const _4 = "._j7hqp8mc{animation-name:kkfkj0m}";
const _3 = "._tip812c5{animation-iteration-count:infinite}";
const _2 = "._5sagi11n{animation-duration:3s}";
const _ = "@keyframes kkfkj0m{0%,33%{box-shadow:var(--_ztiv7h),0 0 0 var(--_khg15c)}66%,to{box-shadow:var(--_ztiv7h),0 0 0 10px #6554c003}}";
const reduceMotionAsPerUserPreference = null;
const baseShadow = `0 0 0 2px ${token('color.border.discovery')}`;
const easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';
const pulseKeyframes = null;
const animationStyles = null;
const Base = ({
  bgColor,
  children,
  className,
  radius,
  testId,
  style,
  // The rest of these props are from `HTMLDivElement`
  ...props
}) => jsx("div", {
  className: className,
  "data-testid": testId,
  style: {
    ...style,
    backgroundColor: bgColor,
    borderRadius: radius ? `${radius}px` : undefined
  },
  ...props,
  children: children
});
export const TargetInner = ({
  bgColor,
  children,
  className,
  pulse,
  radius,
  testId,
  // Thes rest of these are from `HTMLDivElement`
  ...props
}) => jsxs(CC, {
  children: [jsx(CS, {
    children: [_, _2, _3, _4, _5, _6, _7]
  }), jsx(Base, {
    bgColor: bgColor,
    className: ax([pulse && "", pulse && "_5sagi11n _tip812c5 _j7hqp8mc _1pglmcjr _16qs1e2z", "_1bumglyw _sedtglyw", className]),
    radius: radius,
    testId: testId,
    ...props,
    style: {
      ...props.style,
      "--_ztiv7h": ix(baseShadow),
      "--_khg15c": ix(token('color.border.discovery'))
    },
    children: children
  })]
});
