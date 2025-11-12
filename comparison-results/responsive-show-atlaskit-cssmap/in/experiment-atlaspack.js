/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Show", function() {
    return Show;
});
var _react = require("@compiled/react");
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
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
var Show = function(param) {
    var children = param.children;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        css: [
            styles.default
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/responsive-show-atlaskit-cssmap/in.jsx",
            lineNumber: 24,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
