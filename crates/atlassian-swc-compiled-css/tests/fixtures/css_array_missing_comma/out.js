import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _2 = "._30l313q2:hover{color:blue}";
const _ = "._syaz5scu{color:red}";
const baseStyles = {
  color: 'red'
};
const hoverStyles = {
  '&:hover': {
    color: 'blue'
  }
};
export const Component = ({
  isActive,
  children
}) => {
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2]
    }), jsx("div", {
      className: ax(["_syaz5scu", isActive && "_30l313q2"]),
      children: children
    })]
  });
};
