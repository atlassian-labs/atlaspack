/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { css } from '@compiled/react';
import { jsx } from '@atlaskit/css';
const CSS_VAR_ICON_COLOR = '--flag-icon-color';
const descriptionStyles = css({
	maxHeight: 100,
	font: 'normal 14px/1.42857 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Oxygen,Ubuntu,Fira Sans,Droid Sans,Helvetica Neue,sans-serif',
	overflow: 'auto',
	overflowWrap: 'anywhere',
});
const iconWrapperStyles = css({
	display: 'flex',
	minWidth: '24px',
	minHeight: '24px',
	alignItems: 'center',
	justifyContent: 'center',
	flexShrink: 0,
	color: `var(${CSS_VAR_ICON_COLOR})`,
});
const flagWrapperStyles = css({
	width: '100%',
});

const Flag = ({ description, testId }) => {
	return (
		<div role="alert" css={flagWrapperStyles} data-testid={testId}>
			<div css={iconWrapperStyles}>
				Icon
			</div>
			<div css={descriptionStyles}>
				{description}
			</div>
		</div>
	);
};

export default Flag;