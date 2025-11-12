import React from 'react';
import { SimpleTag as Tag } from '@atlaskit/tag';
import { cssMap } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
const styles = cssMap({
    tag: {
        display: 'inline-block',
        padding: '4px 8px',
        borderRadius: '3px',
        fontSize: '12px',
        fontWeight: 'bold',
        textTransform: 'uppercase',
        border: '1px solid #ccc'
    }
});
export function TagComponent({ children, color }) {
    return jsx(Tag, {
        css: styles.tag,
        color: color,
        children: children
    });
}
