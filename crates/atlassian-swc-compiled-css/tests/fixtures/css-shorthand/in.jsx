import { css } from '@compiled/react';

const styles = css({
	outline: '10px solid red',
	border: '1px dashed #000',
	borderColor: 'red green',
	borderWidth: '1px 2px 3px 4px',
	overflow: 'hidden auto',
	gap: '10px 20px',
	columns: '200px 3',
	textDecoration: 'underline wavy #123 2px',
	listStyle: 'square inside url("a.png")',
	inset: '1px 2px 3px 4px',
	marginInline: '10px 20px',
	paddingBlock: '5px 6px',
	borderTop: '2px solid blue',
	borderRight: 0,
	scrollMargin: '1px 2px 3px 4px',
	scrollMarginBlock: '5px 6px',
	scrollPadding: '7px 8px 9px 10px',
	scrollPaddingInline: '11px 12px',
	overscrollBehavior: 'contain auto',
	placeContent: 'center space-between',
	placeItems: 'start end',
	placeSelf: 'stretch center',
	textWrap: 'balance pretty',
	container: 'layout inline-size',
	containIntrinsicSize: '100px 200px',
	scrollTimeline: 'my-tl x',
	viewTimeline: 'my-view block',
});

<div css={styles} />;
