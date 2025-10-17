import { css } from '@compiled/react';

export const View = () => (
	<div
		css={css`
			height: ${Math.random() + 'px'};
			width: ${Math.random() + 'px'};
		`}
	/>
);
