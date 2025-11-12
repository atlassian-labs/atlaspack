import { css } from '@compiled/react';

const theme = {
  primary: '#007bff',
  secondary: '#6c757d',
  spacing: {
    small: '8px',
    medium: '16px',
    large: '24px',
  },
};

const buttonStyles = css`
  background-color: ${theme.primary};
  color: white;
  padding: ${theme.spacing.medium};
  border: 2px solid ${theme.primary};
  border-radius: 4px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
  
  &:hover {
    background-color: ${theme.secondary};
    border-color: ${theme.secondary};
  }
  
  &:focus {
    outline: 2px solid ${theme.primary};
    outline-offset: 2px;
  }
`;

export const Component = ({ children, onClick }) => {
  return (
    <button css={buttonStyles} onClick={onClick}>
      {children}
    </button>
  );
};