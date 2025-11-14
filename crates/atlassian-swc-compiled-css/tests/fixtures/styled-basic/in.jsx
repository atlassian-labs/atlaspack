import { styled } from '@compiled/react';

const Button = styled.div({
	color: 'red',
	'&:hover': { color: 'blue' },
});

export const Btn = () => <Button />;
