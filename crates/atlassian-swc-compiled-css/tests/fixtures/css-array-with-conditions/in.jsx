import { css } from '@compiled/react';

const baseStyles = css({
  display: 'flex',
  padding: '8px',
});

const variantStyles = css({
  primary: {
    backgroundColor: 'blue',
    color: 'white',
  },
  secondary: {
    backgroundColor: 'gray',
    color: 'black',
  },
});

export const Component = ({ variant, disabled, children }) => {
  return (
    <div
      css={[
        baseStyles,
        variant && variantStyles[variant],
        disabled && { opacity: 0.5, pointerEvents: 'none' }
      ]}
    >
      {children}
    </div>
  );
};