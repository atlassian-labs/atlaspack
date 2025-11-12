import { ax, ix, CC, CS } from "@compiled/react/runtime";
import React, { forwardRef } from 'react';
import { jsx } from "react/jsx-runtime";
const styles = {
  root: "_19pkidpf _2hwxidpf _otyridpf _18u0idpf",
  'textAlign.center': "_y3gn1h6o",
  'textAlign.end': "_y3gnh9n0",
  'textAlign.start': "_y3gnv2br"
};
const fontSizeMap = {
  small: "_1wyb1crf",
  medium: "_1wyb7vkz",
  large: "_1wyb1tcg"
};

/**
 * __MetricText__
 *
 * MetricText is a primitive component that displays metrics with different sizes and alignments.
 */
const MetricText = forwardRef((props, ref) => {
  const {
    as: Component = 'span',
    align,
    testId,
    id,
    size,
    children
  } = props;
  return jsx(Component, {
    ref: ref,
    className: `
				${styles.root()}
				${size ? fontSizeMap[size]() : ''}
				${align ? styles[`textAlign.${align}`]() : ''}
			`.trim(),
    "data-testid": testId,
    id: id,
    children: children
  });
});
MetricText.displayName = 'MetricText';
export default MetricText;
