import { styled } from '@compiled/react';

export const Complex = styled.div({
	width: (p) => p.dim.width + 10 + 'px',
	minWidth: (props) => props.width,
	maxWidth: ({ width }) => width,
	height: (p) => p.dim.height,
	backgroundColor: 'pink',
});

export const View = () => <Complex />;
