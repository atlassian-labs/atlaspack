// @flow strict-local

import {globSync} from '@atlaspack/utils';
import {NodeFS} from '@atlaspack/fs';
import rimraf from 'rimraf';

afterEach(() => {
  const fs = new NodeFS();

  globSync(
    'packages/core/integration-tests/**/.{parcel,atlaspack}-cache',
    fs,
  ).forEach((dir) => {
    rimraf.sync(dir);
  });

  globSync('tmp/**/.{parcel,atlaspack}-cache', fs).forEach((dir) => {
    rimraf.sync(dir);
  });
});
