var _taggedTemplateLiteral = require("@swc/helpers/_/_tagged_template_literal");
var _react = require("@compiled/react");
function _templateObject() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n  background-color: ",
        ";\n  color: white;\n  padding: 8px 16px;\n  border: none;\n  border-radius: 4px;\n  font-size: 14px;\n  \n  &:hover {\n    background-color: ",
        ";\n  }\n  \n  &:disabled {\n    opacity: 0.5;\n    cursor: not-allowed;\n  }\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var StyledButton = (0, _react.styled).button(_templateObject(), function(props) {
    return props.primary ? 'blue' : 'gray';
}, function(props) {
    return props.primary ? 'darkblue' : 'darkgray';
});
function App() {
    return /*#__PURE__*/ React.createElement("div", {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-template-dynamic/in.jsx",
            lineNumber: 23,
            columnNumber: 5
        },
        __self: this
    }, /*#__PURE__*/ React.createElement(StyledButton, {
        primary: true,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-template-dynamic/in.jsx",
            lineNumber: 24,
            columnNumber: 7
        },
        __self: this
    }, "Primary Button"), /*#__PURE__*/ React.createElement(StyledButton, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-template-dynamic/in.jsx",
            lineNumber: 25,
            columnNumber: 7
        },
        __self: this
    }, "Secondary Button"));
}
