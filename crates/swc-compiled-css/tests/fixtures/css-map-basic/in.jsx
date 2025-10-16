import { cssMap } from '@compiled/react';

const variants = cssMap({
	primary: {
		color: 'red',
		backgroundColor: 'white',
		'&:hover': { color: 'blue' },
	},
	secondary: {
		color: 'blue',
		backgroundColor: '#eee',
		'&:hover': { color: 'black' },
	},
});

<>
	<div css={variants.primary} />
	<div css={variants.secondary} />
</>;
