import { styled } from '@compiled/react';

const Container = styled.div({
  '[data-field] + button': {
    minWidth: '128px',
  },
});

export const Component = () => (
  <Container>
    <span data-field />
    <button type="button">Action</button>
  </Container>
);
