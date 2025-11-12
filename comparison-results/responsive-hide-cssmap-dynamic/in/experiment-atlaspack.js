/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Hide", function() {
    return Hide;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = (0, _react.cssMap)({
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
    var above = param.above, below = param.below, children = param.children, tmp = param.as, AsElement = tmp === void 0 ? 'div' : tmp, className = param.className;
    return /*#__PURE__*/ (0, _react.jsx)(AsElement, {
        className: className,
        css: [
            above && styles["above.".concat(above)],
            below && styles["below.".concat(below)]
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/responsive-hide-cssmap-dynamic/in.jsx",
            lineNumber: 22,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
