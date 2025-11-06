import { jsx } from '@atlaskit/css';

const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = {};
const iconWrapperStyles = {};
const flagWrapperStyles = {};
const analyticsAttributes = {
    componentName: 'flag',
    packageName: 'test',
    packageVersion: '1.0.0',
};

function Flag() {
    return (
        <div css={iconWrapperStyles}>
            <span css={descriptionStyles}>Content</span>
            <div css={flagWrapperStyles}>Test</div>
        </div>
    );
}

export default Flag;