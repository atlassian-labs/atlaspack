import { styled } from '@compiled/react';

const ListItem = styled.div({
  '> *': {
    flexBasis: ({ isCompact }) =>
      isCompact ? 'var(--ds-space-200,1pc)' : 'var(--ds-space-500,40px)',
  },
});

export const Component = ({ isCompact }) => (
  <ListItem isCompact={isCompact}>
    <div>Content</div>
  </ListItem>
);
