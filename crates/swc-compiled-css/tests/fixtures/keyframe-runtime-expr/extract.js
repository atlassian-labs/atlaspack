const _2 = '._j7hq2c2m{animation-name:vtlc7hanv2mu}';
const _ =
	'@keyframes vtlc7hanv2mu{from { color: var(--1b1u9h2); opacity: var(--owakg9); } to { color: var(--1q3t0o); opacity: var(--10ie45t); }}';
import { ax, ix } from '@compiled/react/runtime';
const runtime = {
	colors: {
		blue: 'blue',
		indigo: 'indigo',
	},
	enabled: true,
};
const getOpacity = (x) => (runtime.enabled ? x : 1);
const fadeOut = null;
export const View = () => (
	<div
		style={{
			'--1b1u9h2': ix(runtime.colors.blue),
			'--owakg9': ix(getOpacity(0)),
			'--1q3t0o': ix(runtime.colors.indigo),
			'--10ie45t': ix(getOpacity(1)),
		}}
		className={ax(['_j7hq2c2m'])}
	/>
);
