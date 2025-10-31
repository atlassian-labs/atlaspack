import { styled } from '@compiled/react';

const PaddingWrapper = styled.div({
  padding: `4px 8px 8px ${({ isSummaryView }) =>
    isSummaryView ? '0px' : '12px'}`,
});

export const Component = () => <PaddingWrapper isSummaryView={false}>Content</PaddingWrapper>;
