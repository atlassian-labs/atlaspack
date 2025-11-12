/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("@compiled/react");
var _css = require("@atlaskit/css");
var _this = undefined;
var CSS_VAR_ICON_COLOR = '--flag-icon-color';
var descriptionStyles = (0, _react.css)({
    maxHeight: 100,
    font: 'normal 14px/1.42857 -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Oxygen,Ubuntu,Fira Sans,Droid Sans,Helvetica Neue,sans-serif',
    overflow: 'auto',
    overflowWrap: 'anywhere'
});
var iconWrapperStyles = (0, _react.css)({
    display: 'flex',
    minWidth: '24px',
    minHeight: '24px',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    color: "var(".concat(CSS_VAR_ICON_COLOR, ")")
});
var flagWrapperStyles = (0, _react.css)({
    width: '100%'
});
var Flag = function(param) {
    var description = param.description, testId = param.testId;
    return /*#__PURE__*/ (0, _css.jsx)("div", {
        role: "alert",
        css: flagWrapperStyles,
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-null-literal-styles/in.jsx",
            lineNumber: 29,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ (0, _css.jsx)("div", {
        css: iconWrapperStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-null-literal-styles/in.jsx",
            lineNumber: 30,
            columnNumber: 4
        },
        __self: _this
    }, "Icon"), /*#__PURE__*/ (0, _css.jsx)("div", {
        css: descriptionStyles,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-null-literal-styles/in.jsx",
            lineNumber: 33,
            columnNumber: 4
        },
        __self: _this
    }, description));
};
exports.default = Flag;
