import { styled } from '@compiled/react';

const StyledDiv = styled.div`
  color: teal;
  &:hover {
    color: black;
  }
`;

export const Component = () => <StyledDiv>Hover me</StyledDiv>;
