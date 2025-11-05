import { css } from '@compiled/react';

const bgColor = 'blue';
const fontSize = 12;
const fontStyling = {
	weight: 500,
};

const sizes = {
	mixin1: () => `1px solid ${bgColor}`,
	mixin2: () => ({ fontSize }),
	mixin3: function () {
		return { fontWeight: fontStyling.weight };
	},
};

const styles2 = css({
	color: 'blue',
	border: sizes.mixin1(),
	...sizes.mixin2(),
	...sizes.mixin3(),
});

<div css={styles2} />;
