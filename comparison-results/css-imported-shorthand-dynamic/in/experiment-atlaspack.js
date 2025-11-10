var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _external = require("./external");
var _this = undefined;
var styles = (0, _react.css)({
    padding: "".concat((0, _external.padding), " ").concat((0, _external.outline), " 0 0")
});
var Component = function() {
    return /*#__PURE__*/ React.createElement("div", {
        css: styles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-imported-shorthand-dynamic/in.jsx",
            lineNumber: 8,
            columnNumber: 32
        },
        __self: _this
    });
};
