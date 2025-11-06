import React from 'react';
import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const Wrapper = styled.div({
	border: ({ isSummaryView }) =>
		isSummaryView ? 'none' : `1px solid ${token('color.border')}`,
});

export const Component = () => <Wrapper isSummaryView={false} />;
