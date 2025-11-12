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
        "\n  color: teal;\n  &:hover {\n    color: black;\n  }\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var StyledDiv = (0, _react.styled).div(_templateObject());
var Component = function() {
    return /*#__PURE__*/ React.createElement(StyledDiv, {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-tagged-template-literal/in.jsx",
            lineNumber: 10,
            columnNumber: 32
        },
        __self: _this
    }, "Hover me");
};
