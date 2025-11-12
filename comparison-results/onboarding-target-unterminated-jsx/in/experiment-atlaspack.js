/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "TargetInner", function() {
    return TargetInner;
});
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _objectWithoutProperties = require("@swc/helpers/_/_object_without_properties");
var _react = require("@compiled/react");
var _tokens = require("./tokens");
var _this = undefined;
var reduceMotionAsPerUserPreference = (0, _react.css)({
    '@media (prefers-reduced-motion: reduce)': {
        animation: 'none',
        transition: 'none'
    }
});
var baseShadow = "0 0 0 2px ".concat((0, _tokens.token)('color.border.discovery'));
var easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';
var pulseKeyframes = (0, _react.keyframes)({
    '0%, 33%': {
        boxShadow: "".concat(baseShadow, ", 0 0 0 ").concat((0, _tokens.token)('color.border.discovery'))
    },
    '66%, 100%': {
        boxShadow: "".concat(baseShadow, ", 0 0 0 10px rgba(101, 84, 192, 0.01)")
    }
});
var animationStyles = (0, _react.css)({
    animationDuration: '3000ms',
    animationIterationCount: 'infinite',
    animationName: pulseKeyframes,
    animationTimingFunction: easing,
    boxShadow: baseShadow
});
var Base = function(_param) {
    var bgColor = _param.bgColor, children = _param.children, className = _param.className, radius = _param.radius, testId = _param.testId, style = _param.style, props = (0, _objectWithoutProperties._)(_param, [
        "bgColor",
        "children",
        "className",
        "radius",
        "testId",
        "style"
    ]);
    return /*#__PURE__*/ (0, _react.jsx)("div", (0, _objectSpreadProps._)((0, _objectSpread._)({
        className: className,
        "data-testid": testId,
        style: (0, _objectSpreadProps._)((0, _objectSpread._)({}, style), {
            backgroundColor: bgColor,
            borderRadius: radius ? "".concat(radius, "px") : undefined
        })
    }, props), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/onboarding-target-unterminated-jsx/in.jsx",
            lineNumber: 45,
            columnNumber: 2
        },
        __self: _this
    }), children);
};
var TargetInner = function(_param) {
    var bgColor = _param.bgColor, children = _param.children, className = _param.className, pulse = _param.pulse, radius = _param.radius, testId = _param.testId, props = (0, _objectWithoutProperties._)(_param, [
        "bgColor",
        "children",
        "className",
        "pulse",
        "radius",
        "testId"
    ]);
    return /*#__PURE__*/ (0, _react.jsx)(Base, (0, _objectSpreadProps._)((0, _objectSpread._)({
        bgColor: bgColor,
        className: className,
        radius: radius,
        testId: testId
    }, props), {
        css: [
            pulse && animationStyles,
            reduceMotionAsPerUserPreference
        ],
        style: props.style,
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/onboarding-target-unterminated-jsx/in.jsx",
            lineNumber: 69,
            columnNumber: 2
        },
        __self: _this
    }), children);
};
