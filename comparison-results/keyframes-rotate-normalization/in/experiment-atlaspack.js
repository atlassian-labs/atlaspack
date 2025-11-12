var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Component", function() {
    return Component;
});
var _react = require("@compiled/react");
var spin = (0, _react.keyframes)({
    '0%': {
        transform: 'rotate(0deg)'
    },
    '100%': {
        transform: 'rotate(360deg)'
    }
});
var Component = (0, _react.styled).div({
    animation: "".concat(spin, " 1.5s linear infinite")
});
