import * as React from 'react';
import { ax, ix, CC, CS } from "@compiled/react/runtime";
import { jsx, jsxs } from "react/jsx-runtime";
const _2 = "._syazwwip{color:#292a2e}";
const _ = "._1e0c1txw{display:flex}";
const styles = {
  container: "_1e0c1txw",
  textBold: "_syazwwip"
};
export const Component = () => jsxs(CC, {
  children: [jsx(CS, {
    children: [_, _2]
  }), jsx("div", {
    className: ax([styles.container]),
    children: jsxs(CC, {
      children: [jsx(CS, {
        children: [_, _2]
      }), jsx("span", {
        className: ax([styles.textBold]),
        children: "Content"
      })]
    })
  })]
});
