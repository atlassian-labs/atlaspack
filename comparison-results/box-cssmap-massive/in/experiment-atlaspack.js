var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
var _defineProperty = require("@swc/helpers/_/_define_property");
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _objectWithoutProperties = require("@swc/helpers/_/_object_without_properties");
var _react = require("react");
var _reactDefault = parcelHelpers.interopDefault(_react);
var _react1 = require("@compiled/react");
var _this = undefined;
var baseStyles = {
    boxSizing: 'border-box',
    appearance: 'none',
    border: 'none'
};
// Massive background color map (simplified from the original 100+ entries)
var backgroundColorMap = (0, _react1.cssMap)({
    'color.background.accent.lime.subtlest': {
        backgroundColor: '#F0F8FF'
    },
    'color.background.accent.lime.subtler': {
        backgroundColor: '#E6F7FF'
    },
    'color.background.accent.lime.subtle': {
        backgroundColor: '#CCE7FF'
    },
    'color.background.accent.red.subtlest': {
        backgroundColor: '#FFF0F0'
    },
    'color.background.accent.red.subtler': {
        backgroundColor: '#FFE6E6'
    },
    'color.background.accent.red.subtle': {
        backgroundColor: '#FFCCCC'
    },
    'color.background.accent.blue.subtlest': {
        backgroundColor: '#F0F8FF'
    },
    'color.background.accent.blue.subtler': {
        backgroundColor: '#E6F7FF'
    },
    'color.background.accent.blue.subtle': {
        backgroundColor: '#CCE7FF'
    },
    'color.background.neutral': {
        backgroundColor: '#F4F5F7'
    },
    'color.background.neutral.hovered': {
        backgroundColor: '#EAECF0'
    },
    'color.background.selected': {
        backgroundColor: '#EBF5FF'
    },
    'elevation.surface': {
        backgroundColor: '#FFFFFF'
    },
    'elevation.surface.raised': {
        backgroundColor: '#FFFFFF'
    }
});
// CSS variables using unboundedCssMap
var CURRENT_SURFACE_CSS_VAR = '--ds-elevation-surface-current';
var setSurfaceTokenMap = (0, _react1.cssMap)({
    'elevation.surface': (0, _defineProperty._)({}, CURRENT_SURFACE_CSS_VAR, '#FFFFFF'),
    'elevation.surface.raised': (0, _defineProperty._)({}, CURRENT_SURFACE_CSS_VAR, '#FFFFFF')
});
// Multiple padding maps
var paddingBlockStartMap = (0, _react1.cssMap)({
    'space.0': {
        paddingBlockStart: '0px'
    },
    'space.100': {
        paddingBlockStart: '8px'
    },
    'space.200': {
        paddingBlockStart: '16px'
    },
    'space.300': {
        paddingBlockStart: '24px'
    }
});
var paddingInlineStartMap = (0, _react1.cssMap)({
    'space.0': {
        paddingInlineStart: '0px'
    },
    'space.100': {
        paddingInlineStart: '8px'
    },
    'space.200': {
        paddingInlineStart: '16px'
    },
    'space.300': {
        paddingInlineStart: '24px'
    }
});
/**
 * __Box__
 *
 * A Box primitive component with massive cssMap configurations
 */ var Box = /*#__PURE__*/ (0, _react.forwardRef)(function(props, ref) {
    var tmp = props.as, Component = tmp === void 0 ? 'div' : tmp, children = props.children, backgroundColor = props.backgroundColor, paddingBlockStart = props.paddingBlockStart, paddingInlineStart = props.paddingInlineStart, style = props.style, testId = props.testId, xcss = props.xcss, htmlAttributes = (0, _objectWithoutProperties._)(props, [
        "as",
        "children",
        "backgroundColor",
        "paddingBlockStart",
        "paddingInlineStart",
        "style",
        "testId",
        "xcss"
    ]);
    var _spreadClass = htmlAttributes.className, safeHtmlAttributes = (0, _objectWithoutProperties._)(htmlAttributes, [
        "className"
    ]);
    var isSurfaceToken = function(bg) {
        return bg in setSurfaceTokenMap;
    };
    return /*#__PURE__*/ (0, _reactDefault.default).createElement(Component, (0, _objectSpreadProps._)((0, _objectSpread._)({
        style: style,
        ref: ref,
        className: "\n				".concat(xcss || '', "\n				").concat(Object.keys(baseStyles).map(function(key) {
            return "".concat(key, ": ").concat(baseStyles[key]);
        }).join('; '), "\n				").concat(backgroundColor ? backgroundColorMap[backgroundColor]() : '', "\n				").concat(backgroundColor && isSurfaceToken(backgroundColor) ? setSurfaceTokenMap[backgroundColor]() : '', "\n				").concat(paddingBlockStart ? paddingBlockStartMap[paddingBlockStart]() : '', "\n				").concat(paddingInlineStart ? paddingInlineStartMap[paddingInlineStart]() : '', "\n			").trim()
    }, safeHtmlAttributes), {
        "data-testid": testId,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/box-cssmap-massive/in.jsx",
            lineNumber: 78,
            columnNumber: 3
        },
        __self: _this
    }), children);
});
Box.displayName = 'Box';
exports.default = Box;
