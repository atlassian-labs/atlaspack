import { styled } from '@compiled/react';

const Base = ({ children }) => <button>{children}</button>;

export const StyledButton = styled(Base)({
  color: 'hotpink',
});

export const Component = () => <StyledButton>Click me</StyledButton>;
