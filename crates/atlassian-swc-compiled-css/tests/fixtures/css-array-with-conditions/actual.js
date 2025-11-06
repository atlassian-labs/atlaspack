import { jsx } from "react/jsx-runtime";
const baseStyles = null;
const variantStyles = null;
export const Component = ({ variant, disabled, children })=>{
    return jsx("div", {
        css: [
            baseStyles,
            variant && variantStyles[variant],
            disabled && {
                opacity: 0.5,
                pointerEvents: 'none'
            }
        ],
        children: children
    });
};
