/** @jsx jsx */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("@compiled/react");
var titleStyles = (0, _react.cssMap)({
    root: {
        marginBottom: '8px'
    }
});
var FeatureCard = (0, _react.styled).div({
    marginTop: '24px'
});
var ButtonContainer = (0, _react.styled).div({
    marginTop: '16px'
});
function FeatureCardView(param) {
    var title = param.title, description = param.description;
    return /*#__PURE__*/ (0, _react.jsx)(FeatureCard, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 18,
            columnNumber: 5
        },
        __self: this
    }, /*#__PURE__*/ (0, _react.jsx)("div", {
        css: titleStyles.root,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 19,
            columnNumber: 7
        },
        __self: this
    }, /*#__PURE__*/ (0, _react.jsx)("h3", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 20,
            columnNumber: 9
        },
        __self: this
    }, title)), /*#__PURE__*/ (0, _react.jsx)("div", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 22,
            columnNumber: 7
        },
        __self: this
    }, description), /*#__PURE__*/ (0, _react.jsx)(ButtonContainer, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 23,
            columnNumber: 7
        },
        __self: this
    }, /*#__PURE__*/ (0, _react.jsx)("button", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/feature-card-cssmap-styled/in.jsx",
            lineNumber: 24,
            columnNumber: 9
        },
        __self: this
    }, "Learn More")));
}
exports.default = FeatureCardView;
