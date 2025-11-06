import { cssMap } from '@atlaskit/css';
import { jsx } from "react/jsx-runtime";
const styles = cssMap({
    default: {
        display: 'none'
    },
    'above.xs': {
        '@media (min-width: 30rem)': {
            display: 'revert'
        }
    },
    'above.sm': {
        '@media (min-width: 48rem)': {
            display: 'revert'
        }
    },
    'above.md': {
        '@media (min-width: 64rem)': {
            display: 'revert'
        }
    },
    'below.xs': {
        '@media not all and (min-width: 30rem)': {
            display: 'revert'
        }
    },
    'below.sm': {
        '@media not all and (min-width: 48rem)': {
            display: 'revert'
        }
    }
});
export const Component = ({ above, below, children })=>{
    return jsx("div", {
        css: [
            styles.default,
            above && styles[`above.${above}`],
            below && styles[`below.${below}`]
        ],
        children: children
    });
};
