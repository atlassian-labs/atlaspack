import { jsx } from "react/jsx-runtime";
const baseShadow = '0 0 0 2px #6554C0';
const easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';
const pulseKeyframes = null;
const reduceMotionAsPerUserPreference = null;
const animationStyles = null;
export const Pulse = ({ children, pulse = true, ...props })=>jsx("div", {
        css: [
            pulse && animationStyles,
            reduceMotionAsPerUserPreference
        ],
        ...props,
        children: children
    });
