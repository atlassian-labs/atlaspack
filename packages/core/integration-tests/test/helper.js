// @flow strict-local

import path from 'path';
import {globSync} from '@atlaspack/utils';
import {NodeFS} from '@atlaspack/fs';
import fs from 'fs';

afterEach(() => {
  const nodeFS = new NodeFS();

  globSync(path.join(__dirname, '**/.parcel-cache'), nodeFS, {
    onlyDirectories: true,
  }).forEach((dir) => {
    fs.rmSync(dir, {recursive: true});
  });

  globSync(path.join(__dirname, '../../../tmp/**/.parcel-cache'), nodeFS, {
    onlyDirectories: true,
  }).forEach((dir) => {
    fs.rmSync(dir, {recursive: true});
  });
});
