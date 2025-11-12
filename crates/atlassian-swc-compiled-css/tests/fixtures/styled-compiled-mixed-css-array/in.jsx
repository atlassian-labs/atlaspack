/** @jsx jsx */
import React from 'react';
import { css as css2, jsx } from '@compiled/react';
import styled, { css } from 'styled-components';

// Mixed patterns: css2 from @compiled/react and css from styled-components
const referencedObjectsContainerStyles = css2({
	display: 'flex',
	flexWrap: 'wrap',
	gap: '4px',
	maxWidth: '100%',
});

const maxWidth2 = css2({
	maxWidth: '200px',
	overflow: 'hidden',
});

const plainTextStyles = css2({
	display: 'flex',
	alignItems: 'center',
	paddingTop: '2px',
	paddingRight: 0,
	paddingBottom: '2px',
	paddingLeft: 0,
	marginRight: '4px',
	backgroundColor: 'inherit',
	color: '#333',
});

// Styled component using css template literal
const LozengeLink = styled.a`
	${lozengeStyles};

	&:focus,
	&:hover {
		background-color: #f4f5f7;
		color: #333;
		text-decoration: none;
	}

	&:active {
		background-color: #e4e5ea;
	}
`;

const lozengeStyles = css`
	display: flex;
	align-items: center;
	padding: 2px 6px;
	border-radius: 3px;
	background-color: #f7f8f9;
`;

// Component using @compiled/react jsx and css
const Component = ({ children, forceMaxWidth }) => (
	<div css={[plainTextStyles, forceMaxWidth && maxWidth2]}>
		{children}
	</div>
);

// Component using styled-components
const StyledComponent = styled.div`
	${plainTextStyles};
	${props => props.forceMaxWidth && maxWidth2};
`;

export default Component;