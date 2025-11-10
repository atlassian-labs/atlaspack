var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "styles", function() {
    return styles;
});
var _react = require("@compiled/react");
var styles = (0, _react.cssMap)({
    success: {
        color: '#0b0',
        '&:hover': {
            color: '#060'
        },
        '@media': {
            'screen and (min-width: 500px)': {
                fontSize: '10vw'
            }
        },
        selectors: {
            span: {
                color: 'lightgreen',
                '&:hover': {
                    color: '#090'
                }
            }
        }
    },
    danger: {
        color: 'red',
        '&:hover': {
            color: 'darkred'
        },
        '@media': {
            'screen and (min-width: 500px)': {
                fontSize: '20vw'
            }
        },
        selectors: {
            span: {
                color: 'orange',
                '&:hover': {
                    color: 'pink'
                }
            }
        }
    }
});
