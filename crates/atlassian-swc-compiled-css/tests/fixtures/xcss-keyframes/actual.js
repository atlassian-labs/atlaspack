import React from 'react';
import { xcss } from '@atlaskit/primitives';
const shimmer = null;
const styles = xcss({
    width: '100%',
    animation: `${shimmer} 1s infinite`,
    background: 'red'
});
export const Component = ()=>jsx("div", {
        xcss: styles
    });
