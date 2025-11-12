import { styled } from '@compiled/react';

const ExpiryDateContainer = styled.div({
  '&::first-letter': {
    textTransform: 'uppercase',
  },
  color: (props) => (props.dueInWeek ? 'var(--ds-text-danger,#ae2e24)' : 'inherit'),
});

export const Component = ({ dueInWeek }) => (
  <ExpiryDateContainer dueInWeek={dueInWeek}>Content</ExpiryDateContainer>
);
