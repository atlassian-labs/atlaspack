/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx, css, keyframes } from '@compiled/react';

const baseShadow = '0 0 0 2px #6554C0';
const easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';

const pulseKeyframes = keyframes({
	'0%, 33%': {
		boxShadow: `${baseShadow}, 0 0 0 #6554C0`,
	},
	'66%, 100%': {
		boxShadow: `${baseShadow}, 0 0 0 10px rgba(101, 84, 192, 0.01)`,
	},
});

const reduceMotionAsPerUserPreference = css({
	'@media (prefers-reduced-motion: reduce)': {
		animation: 'none',
		transition: 'none',
	},
});

const animationStyles = css({
	animationDuration: '3000ms',
	animationIterationCount: 'infinite',
	animationName: pulseKeyframes,
	animationTimingFunction: easing,
	boxShadow: baseShadow,
});

export const Pulse = ({ children, pulse = true, ...props }) => (
	<div
		css={[pulse && animationStyles, reduceMotionAsPerUserPreference]}
		{...props}
	>
		{children}
	</div>
);