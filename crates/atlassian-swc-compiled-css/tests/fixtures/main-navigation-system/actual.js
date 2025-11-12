import { Fragment } from 'react';
import { jsx } from "react/jsx-runtime";
const contentHeightWhenFixed = `calc(100vh - var(--n_bnrM, 0px) - var(--n_tNvM, 0px))`;
const contentInsetBlockStart = `calc(var(--n_bnrM, 0px) + var(--n_tNvM, 0px))`;
const mainElementStyles = {
    root: "_nd5l1gzg _1reo1wug _18m91wug _19121cl4 _152timx3 _qwfh1wug _165teqxy _13wn1if8",
    containPaint: "_njlp1t7j"
};
export function Main({ children, testId, id }) {
    return jsx(Fragment, {
        children: jsx("div", {
            id: id,
            "data-layout-slot": true,
            role: "main",
            css: mainElementStyles.root,
            "data-testid": testId,
            children: children
        })
    });
}
