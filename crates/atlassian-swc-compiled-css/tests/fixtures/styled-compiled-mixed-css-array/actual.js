import React from 'react';
import styled, { css } from 'styled-components';
import { jsx } from "react/jsx-runtime";
const referencedObjectsContainerStyles = null;
const maxWidth2 = null;
const plainTextStyles = null;
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
const Component = ({ children, forceMaxWidth })=>jsx("div", {
        css: [
            plainTextStyles,
            forceMaxWidth && maxWidth2
        ],
        children: children
    });
const StyledComponent = styled.div`
	${plainTextStyles};
	${(props)=>props.forceMaxWidth && maxWidth2};
`;
export default Component;
