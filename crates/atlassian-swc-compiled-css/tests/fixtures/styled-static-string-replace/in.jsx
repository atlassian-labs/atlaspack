import { styled } from '@compiled/react';

const fontFamily = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto'";

export const Component = styled.span({
  fontFamily: fontFamily.replace('BlinkMacSystemFont,', ''),
});
