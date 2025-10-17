import { styled } from '@compiled/react';

const C = styled.div`
	& .child:hover {
		color: ${(p) => (p.active ? 'blue' : 'red')};
	}
	font-size: 14px;
`;

export const View = () => <C />;
