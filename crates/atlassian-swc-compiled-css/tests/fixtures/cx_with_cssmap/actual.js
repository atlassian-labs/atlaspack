import React from 'react';
import { cssMap } from '@atlaskit/css';
import { cx } from '@compiled/react';
import { jsx } from "react/jsx-runtime";
const listStyles = cssMap({
    root: {
        alignItems: 'center',
        gap: '4px',
        display: 'flex'
    },
    popupContainer: {
        padding: '8px'
    }
});
export function Component({ children }) {
    return jsx("div", {
        xcss: cx(listStyles.root, listStyles.popupContainer),
        children: children
    });
}
