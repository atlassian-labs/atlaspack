import { css, keyframes } from '@compiled/react';

const runtime = { colors: { blue: 'blue', indigo: 'indigo' }, enabled: true };
const getOpacity = (x) => (runtime.enabled ? x : 1);

const fadeOut = keyframes`
  from { color: ${runtime.colors.blue}; opacity: ${getOpacity(0)}; }
  to { color: ${runtime.colors.indigo}; opacity: ${getOpacity(1)}; }
`;

export const View = () => (
	<div
		css={css`
			animation-name: ${fadeOut};
		`}
	/>
);
