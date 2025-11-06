import React from 'react';
import { styled } from '@compiled/react';
import { layers } from '@atlassian/jira-common-styles/src/main.tsx';

const Wrapper = styled.div({
	zIndex: layers.blanket,
	position: 'fixed',
});

export const Component = () => <Wrapper />;
