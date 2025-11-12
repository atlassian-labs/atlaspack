var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _taggedTemplateLiteral = require("@swc/helpers/_/_tagged_template_literal");
var _react = require("@compiled/react");
var _this = undefined;
function _templateObject() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n  background-color: ",
        ";\n  color: white;\n  padding: ",
        ";\n  border: 2px solid ",
        ";\n  border-radius: 4px;\n  font-size: 14px;\n  cursor: pointer;\n  transition: all 0.2s ease;\n  \n  &:hover {\n    background-color: ",
        ";\n    border-color: ",
        ";\n  }\n  \n  &:focus {\n    outline: 2px solid ",
        ";\n    outline-offset: 2px;\n  }\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var theme = {
    primary: '#007bff',
    secondary: '#6c757d',
    spacing: {
        small: '8px',
        medium: '16px',
        large: '24px'
    }
};
var buttonStyles = (0, _react.css)(_templateObject(), theme.primary, theme.spacing.medium, theme.primary, theme.secondary, theme.secondary, theme.primary);
var Component = function(param) {
    var children = param.children, onClick = param.onClick;
    return /*#__PURE__*/ React.createElement("button", {
        css: buttonStyles,
        onClick: onClick,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/css-template-with-expressions/in.jsx",
            lineNumber: 36,
            columnNumber: 5
        },
        __self: _this
    }, children);
};
