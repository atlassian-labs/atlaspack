import { styled } from '@compiled/react';

const Component = styled.div({
	paddingLeft: 'var(--ds-space-150, 9pt)',
	borderWidth: 'var(--ds-border-width, 1px)',
	borderColor: 'var(--ds-border, #091E4224)',
	'& > span': {
		marginLeft: 'var(--ds-space-100, 8px)',
	},
});

export const Example = () => (
	<Component>
		<span>Child</span>
	</Component>
);
