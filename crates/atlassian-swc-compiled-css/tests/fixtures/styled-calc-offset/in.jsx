import { styled } from '@compiled/react';
import { LAYOUT_OFFSET } from './layout-offset';

const Container = styled.div({
  height: `calc(100vh - ${LAYOUT_OFFSET})`,
});

export const Component = () => <Container>Content</Container>;
