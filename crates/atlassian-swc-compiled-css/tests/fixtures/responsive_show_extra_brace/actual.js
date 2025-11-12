import { jsx } from "react/jsx-runtime";
const styles = {
    default: {
        display: 'none'
    },
    'above.xs': {
        '@media (min-width: 30rem)': {
            display: 'revert'
        }
    },
    'below.sm': {
        '@media not all and (min-width: 48rem)': {
            display: 'revert'
        }
    }
};
export const Show = ({ above, below, children })=>{
    return jsx("div", {
        css: [
            styles.default,
            above && styles[`above.${above}`],
            below && styles[`below.${below}`]
        ],
        children: children
    });
};
