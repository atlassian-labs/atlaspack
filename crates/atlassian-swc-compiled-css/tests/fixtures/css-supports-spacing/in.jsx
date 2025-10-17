import { css } from '@compiled/react';

const styles = css({
	color: 'red',
	'@supports not (-moz-appearance: none)': {
		color: 'blue',
		backgroundColor: 'white',
	},
	'@supports   not   ( -moz-appearance : none )': {
		color: 'blue',
		backgroundColor: 'white',
	},
});

<div css={styles} />;
