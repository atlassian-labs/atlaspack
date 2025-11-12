import React from 'react';
import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const Wrapper = styled.div({
	'> input::placeholder': {
		color: token('color.text'),
		fontWeight: token('font.weight.medium'),
	},
});

export const Component = () => (
	<Wrapper>
		<input placeholder="Example" />
	</Wrapper>
);
