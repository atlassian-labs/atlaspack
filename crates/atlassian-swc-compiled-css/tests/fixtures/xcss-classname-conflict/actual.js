import { Fragment } from 'react';
import { jsx } from "react/jsx-runtime";
const mainElementStyles = {
    root: "_nd5l1gzg _1reo1wug _18m91wug _19121cl4 _152tckbl _qwfh1wug _165tk7wh _13wn1if8"
};
export function Main({ children, xcss, testId }) {
    return jsx(Fragment, {
        children: jsx("div", {
            className: xcss,
            role: "main",
            css: mainElementStyles.root,
            "data-testid": testId,
            children: children
        })
    });
}
