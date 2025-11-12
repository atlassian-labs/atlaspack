import { styled, css } from '@compiled/react';

const tabStyles = css({
  '&:hover': ({ isDraggable }) => ({
    '--display-icon-before': isDraggable ? 'none' : 'flex',
    '--display-drag-handle': isDraggable ? 'flex' : 'none',
  }),
  ...({ isDragging }) => (isDragging ? { opacity: 0.1 } : {}),
});

export const Component = styled.div(tabStyles);
