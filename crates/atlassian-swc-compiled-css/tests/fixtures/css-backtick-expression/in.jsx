import { css } from '@compiled/react';

const BLUE = 'blue';
const SIZE = 2;
const theme = { col: 'red' };
const COLORS = { bg: 'white' };
const getGreen = () => 'green';

const a = css`
	color: ${BLUE};
`;
const b = css`
	border: ${SIZE}px solid ${theme.col};
`;
const c = css`
	background-color: ${getGreen()};
`;
const d = css`
	&:hover {
		color: ${theme.col};
	}
	:focus {
		border-radius: ${SIZE + 6}px;
	}
`;

<>
	<div css={a} />
	<div css={b} />
	<div css={c} />
	<div css={d} />
</>;
