/* eslint-disable react/no-unknown-property */
/* eslint-disable no-undef */

import React from 'react';
import {css} from '@compiled/react';
import {createRoot} from 'react-dom/client';

import Button from '@atlaskit/button/new';

const styles = css({color: 'red'});

const root = createRoot(document.getElementById('app'));

const page = (
  <>
    <h1 data-testid="heading" css={styles}>
      Hello, world!
    </h1>
    <Button testId="button">Click me</Button>
  </>
);

root.render(page);
