import { styled, css } from '@compiled/react';

const styles = {
	default: css({
		color: 'black',
		fontWeight: 400,
	}),
	success: css({
		color: 'green',
		fontWeight: 600,
	}),
	fail: css({
		color: 'red',
		fontWeight: 600,
	}),
	bg: css({
		background: 'white',
		fontWeight: 900,
	}),
};

const Component = styled.div({
	...styles.default,
	...styles.bg,
});

export const View = () => <Component />;
