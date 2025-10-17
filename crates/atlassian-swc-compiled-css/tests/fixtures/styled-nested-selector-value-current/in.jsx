import { styled } from '@compiled/react';

const C = styled.div`
	&:hover {
		color: ${(p) => (p.isActive ? 'blue' : 'red')};
	}
	font-size: 12px;
`;

export const View = () => <C />;
