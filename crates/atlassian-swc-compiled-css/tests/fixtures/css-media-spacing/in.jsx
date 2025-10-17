import { css } from '@compiled/react';

const styles = css({
	color: 'red',
	backgroundColor: 'black',
	borderRadius: 4,
	'@media(min-width: 64rem)': {
		color: 'blue',
		backgroundColor: 'white',
		borderRadius: 8,
	},
	'@media ( min-width : 64rem )': {
		color: 'blue',
		backgroundColor: 'white',
		borderRadius: 8,
	},
});

<div css={styles} />;
