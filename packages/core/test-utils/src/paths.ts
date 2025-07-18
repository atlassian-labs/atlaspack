import * as path from 'node:path';

export const DIR_PKG = path.normalize(path.join(__dirname, '..'));
export const DIR_CONFIG = path.normalize(path.join(DIR_PKG, 'configs'));
export const FILE_CONFIG_NO_REPORTERS = path.normalize(
  path.join(DIR_CONFIG, '.parcelrc-no-reporters'),
);
