var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var gridSize = 8;
var Container = (0, _react.styled).div(function(param) {
    var hideDropdownLabel = param.hideDropdownLabel;
    return {
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        minHeight: "".concat(gridSize * (hideDropdownLabel ? 14 : 17), "px"),
        overflow: 'hidden'
    };
});
var Component = function(param) {
    var hideDropdownLabel = param.hideDropdownLabel;
    return /*#__PURE__*/ React.createElement(Container, {
        hideDropdownLabel: hideDropdownLabel,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-arrow-function-static-dynamic/in.jsx",
            lineNumber: 14,
            columnNumber: 3
        },
        __self: _this
    });
};
