import { cssMap } from '@compiled/react';

const TEN = 10;
const PX = 'px';
const PADDING = `${TEN}${PX}`;
const COLOR = 'red';
const PALETTE = { brand: 'blue', spacing: { sm: 4 } };
const DOUBLE = TEN + TEN;

const variants = cssMap({
	primary: {
		padding: PADDING,
		margin: TEN,
		borderRadius: DOUBLE,
		borderWidth: PALETTE.spacing.sm,
		color: COLOR,
		backgroundColor: PALETTE.brand,
	},
	danger: {
		color: 'crimson',
	},
});

<>
	<div css={variants.primary} />
	<div css={variants.danger} />
</>;
