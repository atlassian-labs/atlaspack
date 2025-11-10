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
        "\n  display: inline-block;\n  padding: 0;\n  padding-left: ",
        "px;\n  padding-right: ",
        "px;\n"
    ]);
    _templateObject = function _templateObject() {
        return data;
    };
    return data;
}
var gridSize = 8;
var HeadingCellWrapper = (0, _react.styled).div(_templateObject(), function(props) {
    return props.first ? gridSize : gridSize / 2;
}, function(props) {
    return props.last ? gridSize : gridSize / 2;
});
var Component = function(param) {
    var first = param.first, last = param.last;
    return /*#__PURE__*/ React.createElement(HeadingCellWrapper, {
        first: first,
        last: last,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-heading-cell-wrapper/in.jsx",
            lineNumber: 13,
            columnNumber: 3
        },
        __self: _this
    }, "content");
};
