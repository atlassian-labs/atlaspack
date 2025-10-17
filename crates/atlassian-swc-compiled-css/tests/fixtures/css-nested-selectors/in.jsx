import { css } from '@compiled/react';

const styles = css({
	color: 'red',
	backgroundColor: 'black',
	borderRadius: 4,
	'&:hover': { color: 'blue', backgroundColor: 'white' },
	'&:focus': { color: 'green', borderRadius: 8 },
	'&:hover &:focus': { color: 'purple', backgroundColor: 'gray' },
});

<div css={styles} />;
