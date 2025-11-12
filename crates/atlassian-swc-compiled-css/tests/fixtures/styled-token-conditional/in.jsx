import React from 'react';
import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

export const Component = (props) => {
  // eslint-disable-next-line @atlaskit/ui-styling-standard/no-styled -- parity with production usage
  const Label = styled.h5({
    color: props.isDisabled ? token('color.text.disabled') : token('color.text'),
  });

  return <Label>text</Label>;
};
