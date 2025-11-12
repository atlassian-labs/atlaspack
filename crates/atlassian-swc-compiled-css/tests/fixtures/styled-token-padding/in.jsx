import React from 'react';
import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const Wrapper = styled.div({
	padding: `${token('space.050')} ${token('space.150')} ${token('space.150')} ${({ padded }) =>
		padded ? token('space.150') : token('space.0')}`,
});

export const Component = () => <Wrapper padded />;
