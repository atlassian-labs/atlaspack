import { jsx } from "react/jsx-runtime";
const baseStyles = {
    color: 'red'
};
const hoverStyles = {
    '&:hover': {
        color: 'blue'
    }
};
export const Component = ({ isActive, children })=>{
    return jsx("div", {
        css: [
            baseStyles,
            isActive && hoverStyles
        ],
        children: children
    });
};
