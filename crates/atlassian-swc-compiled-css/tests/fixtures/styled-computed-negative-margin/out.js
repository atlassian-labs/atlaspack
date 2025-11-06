import { forwardRef } from 'react';
import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _2 = "._4t3i1osq{height:100%}";
const _ = "._18u0nicn{margin-left:-18px}";
const gridSize = 8;
const IssueContainer = forwardRef(({
  as: C = "div",
  style: __cmpls,
  ...__cmplp
}, __cmplr) => {
  if (__cmplp.innerRef) {
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  }
  return jsxs(CC, {
    children: [jsx(CS, {
      children: [_, _2]
    }), jsx(C, {
      ...__cmplp,
      style: __cmpls,
      ref: __cmplr,
      className: ax(["_18u0nicn _4t3i1osq", __cmplp.className])
    })]
  });
});
if (process.env.NODE_ENV !== 'production') {
  IssueContainer.displayName = 'IssueContainer';
}
const Component = () => jsx(IssueContainer, {
  children: jsx("div", {
    children: "Content"
  })
});
export default Component;
