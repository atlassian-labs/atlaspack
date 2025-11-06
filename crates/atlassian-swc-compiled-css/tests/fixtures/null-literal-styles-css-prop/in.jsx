import React from 'react';

const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = null;
const iconWrapperStyles = null;
const flagWrapperStyles = null;
const analyticsAttributes = {
    componentName: 'flag',
    packageName: 'test',
    packageVersion: '1.0.0',
};

function Flag({ children }) {
    return (
        <div css={iconWrapperStyles}>
            <span css={descriptionStyles}>
                {children}
            </span>
        </div>
    );
}

export default Flag;