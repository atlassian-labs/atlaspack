/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Hide", function() {
    return Hide;
});
var _react = require("@compiled/react");
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
    'above.xs': {
        '@media (min-width: 30rem)': {
            display: 'none'
        }
    },
    'above.sm': {
        '@media (min-width: 48rem)': {
            display: 'none'
        }
    },
    'above.md': {
        '@media (min-width: 64rem)': {
            display: 'none'
        }
    },
    'above.lg': {
        '@media (min-width: 90rem)': {
            display: 'none'
        }
    },
    'above.xl': {
        '@media (min-width: 110.5rem)': {
            display: 'none'
        }
    },
    'below.xs': {
        '@media not all and (min-width: 30rem)': {
            display: 'none'
        }
    },
    'below.sm': {
        '@media not all and (min-width: 48rem)': {
            display: 'none'
        }
    },
    'below.md': {
        '@media not all and (min-width: 64rem)': {
            display: 'none'
        }
    },
    'below.lg': {
        '@media not all and (min-width: 90rem)': {
            display: 'none'
        }
    },
    'below.xl': {
        '@media not all and (min-width: 110.5rem)': {
            display: 'none'
        }
    }
});
var Hide = function(param) {
    var children = param.children;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        css: [],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/responsive-hide-unused-cssmap/in.jsx",
            lineNumber: 23,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
