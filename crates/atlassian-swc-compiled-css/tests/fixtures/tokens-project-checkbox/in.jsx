import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const layoutStyles = {
	alignItems: 'center',
	display: 'flex',
	gap: `${token('space.100')}`,
};

const checkboxStyles = {
	...layoutStyles,
	borderBottom: `${token('space.025')} solid ${token('color.border')}`,
	height: `${token('space.500')}`,
	width: '100%',
};

const getBackgroundColor = (checked, disabled) => {
	if (disabled) {
		return token('color.background.accent.gray.subtlest');
	}

	return checked ? token('color.background.accent.blue.subtlest') : 'transparent';
};

const ProjectCheckbox = styled.div({
	...checkboxStyles,
	backgroundColor: ({ checked, disabled }) =>
		getBackgroundColor(checked, disabled),
	input: {
		marginTop: token('space.0'),
		marginRight: token('space.075'),
		marginBottom: token('space.0'),
		marginLeft: token('space.075'),
	},
	label: {
		...layoutStyles,
	},
	p: {
		marginTop: token('space.0'),
		marginRight: token('space.0'),
		marginBottom: token('space.0'),
		marginLeft: token('space.0'),
	},
});

export const Component = () => (
	<ProjectCheckbox checked disabled={false}>
		<label>
			<input type="checkbox" />
			<p>Description</p>
		</label>
	</ProjectCheckbox>
);
