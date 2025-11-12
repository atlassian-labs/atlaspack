import { styled } from '@compiled/react';

const SELECTED_CELL_BOX_SHADOW = '0 0 0 2px var(--ds-border-focused,#388bff) inset';

export const CellContentWrapper = styled.div({
  '> div > div:first-of-type': {
    borderWidth: 0,
    boxShadow: SELECTED_CELL_BOX_SHADOW,
  },
});

