import { css } from '@compiled/react';

const responsiveStyles = css({
  padding: '16px',
  fontSize: '14px',
  '@media (min-width: 768px)': {
    padding: '24px',
    fontSize: '16px',
  },
  '@media (min-width: 1024px)': {
    padding: '32px',
    fontSize: '18px',
    maxWidth: '1200px',
    margin: '0 auto',
  },
  '@media (prefers-reduced-motion: reduce)': {
    transition: 'none',
  },
  '@media (prefers-color-scheme: dark)': {
    backgroundColor: '#333',
    color: '#fff',
  },
});

export const Component = ({ children }) => {
  return <div css={responsiveStyles}>{children}</div>;
};