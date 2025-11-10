var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "GoalIcon", function() {
    return GoalIcon;
});
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _css = require("@atlaskit/css");
var styles = (0, _css.cssMap)({
    goalIcon: {
        borderStyle: 'solid',
        borderRadius: '4px',
        borderColor: '#ccc',
        borderWidth: '1px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center'
    },
    size16: {
        width: '16px',
        height: '16px'
    },
    size24: {
        width: '24px',
        height: '24px'
    },
    size32: {
        width: '32px',
        height: '32px'
    }
});
function GoalIcon(param) {
    var _param_size = param.size, size = _param_size === void 0 ? '24' : _param_size;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement("div", {
        xcss: (0, _css.cx)(styles.goalIcon, styles["size".concat(size)]),
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/cx_with_dynamic_cssmap_property/in.jsx",
            lineNumber: 30,
            columnNumber: 3
        },
        __self: this
    }, "Content");
}
