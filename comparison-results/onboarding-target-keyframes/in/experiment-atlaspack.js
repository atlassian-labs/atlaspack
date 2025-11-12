/**
 * @jsxRuntime classic
 * @jsx jsx
 */ var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
parcelHelpers.defineInteropFlag(exports);
parcelHelpers.export(exports, "Pulse", function() {
    return Pulse;
});
var _objectSpread = require("@swc/helpers/_/_object_spread");
var _objectSpreadProps = require("@swc/helpers/_/_object_spread_props");
var _objectWithoutProperties = require("@swc/helpers/_/_object_without_properties");
var _react = require("@compiled/react");
var _this = undefined;
var baseShadow = '0 0 0 2px #6554C0';
var easing = 'cubic-bezier(0.55, 0.055, 0.675, 0.19)';
var pulseKeyframes = (0, _react.keyframes)({
    '0%, 33%': {
        boxShadow: "".concat(baseShadow, ", 0 0 0 #6554C0")
    },
    '66%, 100%': {
        boxShadow: "".concat(baseShadow, ", 0 0 0 10px rgba(101, 84, 192, 0.01)")
    }
});
var reduceMotionAsPerUserPreference = (0, _react.css)({
    '@media (prefers-reduced-motion: reduce)': {
        animation: 'none',
        transition: 'none'
    }
});
var animationStyles = (0, _react.css)({
    animationDuration: '3000ms',
    animationIterationCount: 'infinite',
    animationName: pulseKeyframes,
    animationTimingFunction: easing,
    boxShadow: baseShadow
});
var Pulse = function(_param) {
    var children = _param.children, _param_pulse = _param.pulse, pulse = _param_pulse === void 0 ? true : _param_pulse, props = (0, _objectWithoutProperties._)(_param, [
        "children",
        "pulse"
    ]);
    return /*#__PURE__*/ (0, _react.jsx)("div", (0, _objectSpreadProps._)((0, _objectSpread._)({
        css: [
            pulse && animationStyles,
            reduceMotionAsPerUserPreference
        ]
    }, props), {
        __source: {
            fileName: "crates/atlassian-swc-compiled-css/tests/fixtures/onboarding-target-keyframes/in.jsx",
            lineNumber: 35,
            columnNumber: 2
        },
        __self: _this
    }), children);
};
