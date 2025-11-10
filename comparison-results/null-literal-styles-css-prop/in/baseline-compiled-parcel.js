var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var CSS_VAR_ICON_COLOR = '--flag-icon-color';
var descriptionStyles = null;
var iconWrapperStyles = null;
var flagWrapperStyles = null;
var analyticsAttributes = {
    componentName: 'flag',
    packageName: 'test',
    packageVersion: '1.0.0'
};
function Flag(param) {
    var children = param.children;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement("div", {
        css: iconWrapperStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/null-literal-styles-css-prop/in.jsx",
            lineNumber: 15,
            columnNumber: 9
        },
        __self: this
    }, /*#__PURE__*/ (0, _reactDefault.default).createElement("span", {
        css: descriptionStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/null-literal-styles-css-prop/in.jsx",
            lineNumber: 16,
            columnNumber: 13
        },
        __self: this
    }, children));
}
exports.default = Flag;
