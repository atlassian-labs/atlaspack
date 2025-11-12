var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _this = undefined;
var styles = (0, _react1.cssMap)({
    root: {
        margin: 0
    },
    'textAlign.center': {
        textAlign: 'center'
    },
    'textAlign.end': {
        textAlign: 'end'
    },
    'textAlign.start': {
        textAlign: 'start'
    }
});
var fontSizeMap = (0, _react1.cssMap)({
    small: {
        fontSize: '12px'
    },
    medium: {
        fontSize: '16px'
    },
    large: {
        fontSize: '24px'
    }
});
/**
 * __MetricText__
 *
 * MetricText is a primitive component that displays metrics with different sizes and alignments.
 */ var MetricText = /*#__PURE__*/ (0, _react.forwardRef)(function(props, ref) {
    var tmp = props.as, Component = tmp === void 0 ? 'span' : tmp, align = props.align, testId = props.testId, id = props.id, size = props.size, children = props.children;
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Component, {
        ref: ref,
        className: "\n				".concat(styles.root(), "\n				").concat(size ? fontSizeMap[size]() : '', "\n				").concat(align ? styles["textAlign.".concat(align)]() : '', "\n			").trim(),
        "data-testid": testId,
        id: id,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/metric-text-cssmap-mixed/in.jsx",
            lineNumber: 36,
            columnNumber: 3
        },
        __self: _this
    }, children);
});
MetricText.displayName = 'MetricText';
exports.default = MetricText;
