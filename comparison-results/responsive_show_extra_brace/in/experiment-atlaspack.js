/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Show", function() {
    return Show;
});
var _react = require("@compiled/react");
var _this = undefined;
var styles = {
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
var Show = function(param) {
    var above = param.above, below = param.below, children = param.children;
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        css: [
            styles.default,
            above && styles["above.".concat(above)],
            below && styles["below.".concat(below)]
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/responsive_show_extra_brace/in.jsx",
            lineNumber: 15,
            columnNumber: 3
        },
        __self: _this
    }, children);
};
