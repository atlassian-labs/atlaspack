/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { cssMap as cssMapUnbounded } from '@compiled/react';
import { cssMap, jsx } from '@atlaskit/css';
import { token } from '@atlaskit/tokens';

const stylesOld = cssMap({
	container: {
		display: 'inline-flex',
		borderRadius: token('radius.small'),
		blockSize: 'min-content',
		position: 'static',
		overflow: 'hidden',
		paddingInline: token('space.050'),
		boxSizing: 'border-box',
	},
	'text.bold.default': { color: token('color.text.inverse', '#FFFFFF') },
	'text.bold.inprogress': { color: token('color.text.inverse', '#FFFFFF') },
	'text.subtle.default': { color: token('color.text.subtle', '#42526E') },
});

const stylesOldUnbounded = cssMapUnbounded({
	text: {
		fontFamily: token('font.family.body'),
		fontSize: '11px',
		fontStyle: 'normal',
		fontWeight: token('font.weight.bold'),
		lineHeight: '16px',
		overflow: 'hidden',
		textOverflow: 'ellipsis',
		textTransform: 'uppercase',
		whiteSpace: 'nowrap',
	},
	customLetterspacing: {
		letterSpacing: 0.165,
	},
});

export const Lozenge = ({ children, appearance = 'default', isBold = false }) => (
	<div css={[stylesOld.container, stylesOldUnbounded.text, stylesOldUnbounded.customLetterspacing]}>
		<span css={stylesOld[`text.${isBold ? 'bold' : 'subtle'}.${appearance}`]}>
			{children}
		</span>
	</div>
);