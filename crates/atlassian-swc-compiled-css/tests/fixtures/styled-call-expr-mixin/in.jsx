import { styled } from '@compiled/react';

const mixin = () => ({ color: 'red' });

const ListItem = styled.div({
	...mixin(),
});

export const View = () => <ListItem />;
