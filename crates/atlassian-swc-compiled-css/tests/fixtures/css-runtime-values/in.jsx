import { css } from '@compiled/react';

export const View = () => (
	<div css={css({ width: Math.random() + 'px', height: Math.random() + 'px' })} />
);
