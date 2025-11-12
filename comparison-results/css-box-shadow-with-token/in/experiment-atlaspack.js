/** @jsx jsx */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("@compiled/react");
var _tokens = require("@atlaskit/tokens");
var _this = undefined;
var border = (0, _react.css)({
    boxShadow: "inset 0 -1px 0 0 ".concat((0, _tokens.token)('color.border'))
});
var Component = function() {
    return /*#__PURE__*/ (0, _react.jsx)("div", {
        css: border,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-box-shadow-with-token/in.jsx",
            lineNumber: 8,
            columnNumber: 3
        },
        __self: _this
    }, /*#__PURE__*/ (0, _react.jsx)("span", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-box-shadow-with-token/in.jsx",
            lineNumber: 9,
            columnNumber: 5
        },
        __self: _this
    }, "Content with border"));
};
exports.default = Component;
