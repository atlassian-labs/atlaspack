// @flow strict-local

import fs from 'fs';

export class IdentifierRegistry {
  constructor() {}

  addIdentifier(type: string, identifier: string, data: mixed) {
    fs.appendFileSync(
      './atlaspack-identifiers.txt',
      // $FlowFixMe
      `${type} ${identifier} ${JSON.stringify(data)}\n`,
    );
  }
}

export const identifierRegistry: IdentifierRegistry = new IdentifierRegistry();
