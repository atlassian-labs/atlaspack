import { styled } from '@compiled/react';

const StyledButton = styled.button`
  background-color: ${props => props.primary ? 'blue' : 'gray'};
  color: white;
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  
  &:hover {
    opacity: 0.8;
  }
  
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
`;

export const Component = ({ primary, disabled, children, ...props }) => {
  return (
    <StyledButton primary={primary} disabled={disabled} {...props}>
      {children}
    </StyledButton>
  );
};