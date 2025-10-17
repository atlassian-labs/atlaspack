import { styled } from '@compiled/react';

const Component = styled.div`
	${(props) => props.isPrimary && { color: 'blue', background: 'blue' }};
	border: 3px solid yellow;
	color: red;
	background: white;
`;

export const View = () => <Component />;
