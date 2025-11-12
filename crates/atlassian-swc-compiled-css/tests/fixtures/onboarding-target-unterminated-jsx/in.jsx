/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx, css, keyframes } from '@compiled/react';
import { token } from './tokens';

const reduceMotionAsPerUserPreference = css({
	'@media (prefers-reduced-motion: reduce)': {
		animation: 'none',
		transition: 'none',
	},
});

const baseShadow = `0 0 0 2px ${token('color.border.discovery')}`;
const easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';

const pulseKeyframes = keyframes({
	'0%, 33%': {
		boxShadow: `${baseShadow}, 0 0 0 ${token('color.border.discovery')}`,
	},
	'66%, 100%': {
		boxShadow: `${baseShadow}, 0 0 0 10px rgba(101, 84, 192, 0.01)`,
	},
});

const animationStyles = css({
	animationDuration: '3000ms',
	animationIterationCount: 'infinite',
	animationName: pulseKeyframes,
	animationTimingFunction: easing,
	boxShadow: baseShadow,
});

const Base = ({
	bgColor,
	children,
	className,
	radius,
	testId,
	style,
	// The rest of these props are from `HTMLDivElement`
	...props
}) => (
	<div
		className={className}
		data-testid={testId}
		style={{
			...style,
			backgroundColor: bgColor,
			borderRadius: radius ? `${radius}px` : undefined,
		}}
		{...props}
	>
		{children}
	</div>
);

export const TargetInner = ({
	bgColor,
	children,
	className,
	pulse,
	radius,
	testId,
	// Thes rest of these are from `HTMLDivElement`
	...props
}) => (
	<Base
		bgColor={bgColor}
		className={className}
		radius={radius}
		testId={testId}
		{...props}
		css={[pulse && animationStyles, reduceMotionAsPerUserPreference]}
		style={props.style}
	>
		{children}
	</Base>
);