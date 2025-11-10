var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var _this = undefined;
var SIZE = 16;
var Icon = (0, _react.styled).div({
    flexBasis: "".concat(SIZE, "px"),
    backgroundImage: function(param) {
        var url = param.url;
        return url ? "url(".concat(url, ")") : 'none';
    }
});
var Component = function(param) {
    var url = param.url;
    return /*#__PURE__*/ React.createElement(Icon, {
        url: url,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/styled-static-and-dynamic/in.jsx",
            lineNumber: 10,
            columnNumber: 39
        },
        __self: _this
    });
};
