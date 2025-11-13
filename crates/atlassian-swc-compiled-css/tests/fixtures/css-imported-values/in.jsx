import { css } from '@compiled/react';
import { value } from 'location';

export const View = () => <div css={css({ width: value, height: value + 'px' })} />;
