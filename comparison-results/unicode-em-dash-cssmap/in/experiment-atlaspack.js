var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "TagComponent", function() {
    return TagComponent;
});
var _tag = require("@atlaskit/tag");
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _css = require("@atlaskit/css");
// eslint-disable-next-line @atlaskit/design-system/no-emotion-primitives – to be migrated to @atlaskit/primitives/compiled
var styles = (0, _css.cssMap)({
    tag: {
        display: 'inline-block',
        padding: '4px 8px',
        borderRadius: '3px',
        fontSize: '12px',
        fontWeight: 'bold',
        textTransform: 'uppercase',
        // Unicode character – causing boundary issues
        border: '1px solid #ccc'
    }
});
function TagComponent(param) {
    var children = param.children, color = param.color;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement((0, _tag.SimpleTag), {
        css: styles.tag,
        color: color,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/unicode-em-dash-cssmap/in.jsx",
            lineNumber: 21,
            columnNumber: 3
        },
        __self: this
    }, children);
}
