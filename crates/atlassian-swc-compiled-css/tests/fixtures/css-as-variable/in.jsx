import { css } from '@compiled/react';
import { value } from 'test';

const test = css({ color: value });
const tt = <div css={test} />;
