var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _css = require("@atlaskit/css");
var _this = undefined;
var styles = (0, _css.cssMap)({
    base: {
        color: 'red',
        '&:hover': {
            color: 'blue'
        }
    }
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        className: styles.base,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-map-atlaskit/in.jsx",
            lineNumber: 12,
            columnNumber: 32
        },
        __self: _this
    }, "hello");
};
