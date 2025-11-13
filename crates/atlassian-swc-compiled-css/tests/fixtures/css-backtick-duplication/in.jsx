import { css } from '@compiled/react';

const obj1 = css({
	color: 'red',
	border: '1px solid green',
	'&:hover': { color: 'blue' },
	'@media (min-width: 64rem)': { gridArea: 'main' },
});

const tpl1 = css`
	color: red;
	border: 1px solid green;
	&:hover {
		color: blue;
	}
	@media (min-width: 64rem) {
		grid-area: main;
	}
`;

<>
	<div css={obj1} />
	<div css={tpl1} />
</>;
