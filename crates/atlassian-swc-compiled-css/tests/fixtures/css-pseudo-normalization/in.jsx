import { css } from '@compiled/react';

const styles = css({
	color: 'red',
	':hover': { color: 'blue' },
	'&:focus': { color: 'green' },
	':after': { content: '""' },
	'::after': { content: '""' },
});

<div css={styles} />;
