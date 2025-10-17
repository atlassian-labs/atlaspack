import { css } from '@compiled/react';

const a = css`
	color: blue;
	border: 1px solid green;
`;
const b = css`
	&:hover {
		color: red;
	}
	:focus {
		border-radius: 8px;
	}
`;

<>
	<div css={a} />
	<div css={b} />
</>;
