import React from 'react';
import { styled } from '@compiled/react';
import { Tokens } from '@atlaskit/tokens';

const Wrapper = styled.div({
  color: ({ withSidebar }) => (withSidebar ? undefined : Tokens.COLOR_TEXT),
});

export const Component = ({ withSidebar }) => <Wrapper withSidebar={withSidebar} />;
