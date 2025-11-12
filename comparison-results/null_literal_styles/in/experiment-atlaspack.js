var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _css = require("@atlaskit/css");
var CSS_VAR_ICON_COLOR = '--flag-icon-color';
var descriptionStyles = {};
var iconWrapperStyles = {};
var flagWrapperStyles = {};
var analyticsAttributes = {
    componentName: 'flag',
    packageName: 'test',
    packageVersion: '1.0.0'
};
function Flag() {
    return /*#__PURE__*/ React.createElement("div", {
        css: iconWrapperStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/null_literal_styles/in.jsx",
            lineNumber: 15,
            columnNumber: 9
        },
        __self: this
    }, /*#__PURE__*/ React.createElement("span", {
        css: descriptionStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/null_literal_styles/in.jsx",
            lineNumber: 16,
            columnNumber: 13
        },
        __self: this
    }, "Content"), /*#__PURE__*/ React.createElement("div", {
        css: flagWrapperStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/null_literal_styles/in.jsx",
            lineNumber: 17,
            columnNumber: 13
        },
        __self: this
    }, "Test"));
}
exports.default = Flag;
