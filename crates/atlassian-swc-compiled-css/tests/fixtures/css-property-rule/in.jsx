import { css } from '@compiled/react';

const styles = css({
	'@property --x': {
		syntax: '<number>',
		inherits: false,
		initialValue: 0,
	},
	color: 'red',
});

<div css={styles} />;
