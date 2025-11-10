import React from 'react';
import {token} from '@atlaskit/tokens';

const Component = () => {
  return <input type="text" placeholder="This has a special characer that should not be munged: …"/>;
};

const v = token('color.text');
const t = Component();
console.log(v, t);
