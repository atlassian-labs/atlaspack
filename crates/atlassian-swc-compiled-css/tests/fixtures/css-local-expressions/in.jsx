import { css } from '@compiled/react';

const TEN = 10;
const PX = 'px';
const PADDING = `${TEN}${PX}`;
const COLOR = 'red';
const PALETTE = { brand: 'blue', spacing: { sm: 4 } };
const DOUBLE = TEN + TEN;

const styles = css({
	padding: PADDING,
	margin: TEN,
	borderRadius: DOUBLE,
	borderWidth: PALETTE.spacing.sm,
	color: COLOR,
	backgroundColor: PALETTE.brand,
});
<div css={styles} />;
