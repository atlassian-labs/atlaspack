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
    'above.lg': {
        '@media (min-width: 90rem)': {
            display: 'revert'
        }
    },
    'above.xl': {
        '@media (min-width: 110.5rem)': {
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
    },
    'below.md': {
        '@media not all and (min-width: 64rem)': {
            display: 'revert'
        }
    },
    'below.lg': {
        '@media not all and (min-width: 90rem)': {
            display: 'revert'
        }
    },
    'below.xl': {
        '@media not all and (min-width: 110.5rem)': {
            display: 'revert'
        }
    }
});
export const Show = ({ children })=>{
    return jsx("div", {
        css: [
            styles.default
        ],
        children: children
    });
};
