import React from 'react';
import { Box } from '@atlaskit/primitives/compiled';
import { cssMap } from '@atlaskit/css';
import { jsx, jsxs } from "react/jsx-runtime";
const styles = cssMap({
    avatarItemWrapper: {
        marginLeft: '-6px',
        paddingRight: '8px'
    },
    container: {
        display: 'flex',
        alignItems: 'center',
        backgroundColor: '#f4f5f7'
    },
    text: {
        fontSize: '14px',
        fontWeight: 'bold',
        color: '#172b4d'
    }
});
export const Component = ({ name, picture })=>{
    return jsx(Box, {
        xcss: styles.avatarItemWrapper,
        children: jsxs("div", {
            className: styles.container(),
            children: [
                jsx("img", {
                    src: picture,
                    alt: name
                }),
                jsx("span", {
                    className: styles.text(),
                    children: name
                })
            ]
        })
    });
};
export default Component;
