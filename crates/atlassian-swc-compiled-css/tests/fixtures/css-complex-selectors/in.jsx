import { css } from '@compiled/react';

const styles = css`
	> span:first-type-of {
		color: red;
		&:hover {
			color: blue;
		}
	}
`;

<div css={styles} />;
