var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _objectWithoutProperties = require("@swc/helpers/_/_object_without_properties");
var _taggedTemplateLiteral = require("@swc/helpers/_/_tagged_template_literal");
var _react = require("@compiled/react");
var _this = undefined;
function _templateObject() {
    var data = (0, _taggedTemplateLiteral._)([
        "\n  background-color: ",
        ";\n  color: white;\n  padding: 8px 16px;\n  border: none;\n  border-radius: 4px;\n  cursor: pointer;\n  \n  &:hover {\n    opacity: 0.8;\n  }\n  \n  &:disabled {\n    opacity: 0.5;\n    cursor: not-allowed;\n  }\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var StyledButton = (0, _react.styled).button(_templateObject(), function(props) {
    return props.primary ? 'blue' : 'gray';
});
var Component = function(_param) {
    var primary = _param.primary, disabled = _param.disabled, children = _param.children, props = (0, _objectWithoutProperties._)(_param, [
        "primary",
        "disabled",
        "children"
    ]);
    return /*#__PURE__*/ React.createElement(StyledButton, (0, _objectSpreadProps._)((0, _objectSpread._)({
        primary: primary,
        disabled: disabled
    }, props), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-component-basic/in.jsx",
            lineNumber: 23,
            columnNumber: 5
        },
        __self: _this
    }), children);
};
