import { styled } from '@compiled/react';

const C = styled.div`
	@media (min-width: 600px) {
		color: ${(p) => (p.isPrimary ? 'blue' : 'red')};
	}
	font-size: 10px;
`;

export const View = () => <C />;
