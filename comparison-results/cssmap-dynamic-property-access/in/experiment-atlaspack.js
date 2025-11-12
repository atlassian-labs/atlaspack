var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
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
var Component = function(param) {
    var above = param.above, below = param.below, children = param.children;
    return /*#__PURE__*/ React.createElement("div", {
        css: [
            styles.default,
            above && styles["above.".concat(above)],
            below && styles["below.".concat(below)]
        ],
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cssmap-dynamic-property-access/in.jsx",
            lineNumber: 15,
            columnNumber: 5
        },
        __self: _this
    }, children);
};
