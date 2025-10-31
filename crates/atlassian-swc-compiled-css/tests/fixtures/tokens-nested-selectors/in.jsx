import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const Wrapper = styled.div({
	input: {
		marginTop: token('space.0'),
		marginRight: token('space.075'),
		marginBottom: token('space.0'),
		marginLeft: token('space.075'),
	},
	label: {
		color: token('color.text.subtle'),
	},
});

export const Component = () => (
	<Wrapper>
		<label>
			<input type="checkbox" />
		</label>
	</Wrapper>
);
