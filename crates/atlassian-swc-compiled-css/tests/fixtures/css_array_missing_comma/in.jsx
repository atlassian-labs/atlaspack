/**
 * @jsxRuntime classic
 * @jsx jsx
 */
import { jsx } from '@compiled/react';

const baseStyles = { color: 'red' };
const hoverStyles = { '&:hover': { color: 'blue' } };

export const Component = ({ isActive, children }) => {
	return (
		<div
			css={[baseStyles, isActive && hoverStyles]}
		>
			{children}
		</div>
	);
};