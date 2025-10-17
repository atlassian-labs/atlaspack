import { styled, css } from '@compiled/react';

const dark = css`
	background-color: black;
	color: white;
`;

const light = css({
	'background-color': 'white',
	color: 'black',
});

const Component = styled.div`
	${(p) => (p.isDark ? dark : light)};
	font-size: 30px;
`;

export const View = () => <Component />;
