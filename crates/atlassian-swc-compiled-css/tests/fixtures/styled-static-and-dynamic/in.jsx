import { styled } from '@compiled/react';

const SIZE = 16;

const Icon = styled.div({
  flexBasis: `${SIZE}px`,
  backgroundImage: ({ url }) => (url ? `url(${url})` : 'none'),
});

export const Component = ({ url }) => <Icon url={url} />;
