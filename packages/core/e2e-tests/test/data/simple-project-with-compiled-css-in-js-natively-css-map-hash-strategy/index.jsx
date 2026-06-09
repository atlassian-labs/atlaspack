/* eslint-disable react/no-unknown-property */
/* eslint-disable no-undef */

import React from 'react';
import {cssMap} from '@compiled/react';
import {createRoot} from 'react-dom/client';

import Button from '@atlaskit/button/new';

// @experimental: hashStrategy is not part of the public API.
const styles = cssMap(
  {
    danger: {color: 'red'},
  },
  {hashStrategy: 'max'},
);

const root = createRoot(document.getElementById('app'));

const page = (
  <>
    <h1 data-testid="heading" css={styles.danger}>
      Hello, world!
    </h1>
    <Button testId="button">Click me</Button>
  </>
);

root.render(page);
