import { forwardRef } from 'react';
import * as React from 'react';
import { ax, ix } from "@compiled/react/runtime";
const SELECTED_CELL_BOX_SHADOW = '0 0 0 2px var(--ds-border-focused,#388bff) inset';
export const CellContentWrapper = forwardRef(({
  as: C = "div",
  style: __cmpls,
  ...__cmplp
}, __cmplr) => {
  if (__cmplp.innerRef) {
    throw new Error("Please use 'ref' instead of 'innerRef'.");
  }
  return <C {...__cmplp} style={__cmpls} ref={__cmplr} className={ax(["c_CellContentWrapper", "_1p3lidpf _13ce4ls2", __cmplp.className])} />;
});
if (process.env.NODE_ENV !== 'production') {
  CellContentWrapper.displayName = 'CellContentWrapper';
}
