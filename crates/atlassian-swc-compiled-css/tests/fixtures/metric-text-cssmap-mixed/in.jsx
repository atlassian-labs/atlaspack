import React, { forwardRef } from 'react';
import { jsx, cssMap as unboundedCssMap } from '@compiled/react';
import { cssMap } from '@compiled/react';

const styles = unboundedCssMap({
	root: {
		margin: 0,
	},
	'textAlign.center': { textAlign: 'center' },
	'textAlign.end': { textAlign: 'end' },
	'textAlign.start': { textAlign: 'start' },
});

const fontSizeMap = cssMap({
	small: { fontSize: '12px' },
	medium: { fontSize: '16px' },
	large: { fontSize: '24px' },
});

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

	return (
		<Component
			ref={ref}
			className={`
				${styles.root()}
				${size ? fontSizeMap[size]() : ''}
				${align ? styles[`textAlign.${align}`]() : ''}
			`.trim()}
			data-testid={testId}
			id={id}
		>
			{children}
		</Component>
	);
});

MetricText.displayName = 'MetricText';

export default MetricText;