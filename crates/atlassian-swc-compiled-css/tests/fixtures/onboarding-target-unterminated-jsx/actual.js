import { token } from './tokens';
import { jsx } from "react/jsx-runtime";
const reduceMotionAsPerUserPreference = null;
const baseShadow = `0 0 0 2px ${"var(--ds-border-discovery, #8270DB)"}`;
const easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';
const pulseKeyframes = null;
const animationStyles = null;
const Base = ({ bgColor, children, className, radius, testId, style, ...props })=>jsx("div", {
        className: className,
        "data-testid": testId,
        style: {
            ...style,
            backgroundColor: bgColor,
            borderRadius: radius ? `${radius}px` : undefined
        },
        ...props,
        children: children
    });
export const TargetInner = ({ bgColor, children, className, pulse, radius, testId, ...props })=>jsx(Base, {
        bgColor: bgColor,
        className: className,
        radius: radius,
        testId: testId,
        ...props,
        css: [
            pulse && animationStyles,
            reduceMotionAsPerUserPreference
        ],
        style: props.style,
        children: children
    });
