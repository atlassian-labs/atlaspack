import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _4 = "@media not all and (min-width:48rem){._4lz619ly below.sm{display:revert}}";
const _3 = "@media (min-width:30rem){._spsm19ly above.xs{display:revert}}";
const _2 = "._rs4eglyw default{display:none}";
const _ = "._1e0cglyw{display:none}";
const styles = {
  default: {
    display: 'none'
  },
  'above.xs': {
    '@media (min-width: 30rem)': {
      display: 'revert'
    }
  },
  'below.sm': {
    '@media not all and (min-width: 48rem)': {
      display: 'revert'
    }
  }
};
export const Show = ({
  above,
  below,
  children
}) => {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2, _3, _4]
    }), jsx("div", {
      className: ax(["_1e0cglyw", above && "_rs4eglyw _spsm19ly _4lz619ly", below && "_rs4eglyw _spsm19ly _4lz619ly"]),
      children: children
    })]
  });
};
