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
        "\n  padding: ",
        ";\n  padding-left: ",
        "px;\n  padding-right: ",
        "px;\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var padding = '8px';
var large = 8;
var small = 4;
var Cell = (0, _react.styled).div(_templateObject(), padding, function(props) {
    return props.first ? large : small;
}, function(props) {
    return props.last ? large : small;
});
var Component = function(param) {
    var first = param.first, last = param.last;
    return /*#__PURE__*/ React.createElement(Cell, {
        first: first,
        last: last,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-template-conditional/in.jsx",
            lineNumber: 14,
            columnNumber: 3
        },
        __self: _this
    }, "Content");
};
