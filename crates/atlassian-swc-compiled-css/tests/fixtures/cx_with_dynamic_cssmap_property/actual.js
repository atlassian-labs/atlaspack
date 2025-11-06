import React from 'react';
import { cssMap, cx } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
const styles = cssMap({
    goalIcon: {
        borderStyle: 'solid',
        borderRadius: '4px',
        borderColor: '#ccc',
        borderWidth: '1px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center'
    },
    size16: {
        width: '16px',
        height: '16px'
    },
    size24: {
        width: '24px',
        height: '24px'
    },
    size32: {
        width: '32px',
        height: '32px'
    }
});
export function GoalIcon({ size = '24' }) {
    return jsx("div", {
        xcss: cx(styles.goalIcon, styles[`size${size}`]),
        children: "Content"
    });
}
