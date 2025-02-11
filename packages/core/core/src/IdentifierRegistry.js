// @flow strict-local

import fs from 'fs';

export class IdentifierRegistry {
  #createdDirectory = false;

  constructor() {}

  addIdentifier(type: string, identifier: string, data: mixed) {
    if (process.env.ATLASPACK_IDENTIFIER_DEBUG === 'true') {
      if (!this.#createdDirectory) {
        fs.mkdirSync('./.atlaspack', {recursive: true});
        this.#createdDirectory = true;
      }

      fs.appendFile(
        './.atlaspack/atlaspack-identifiers.txt',
        // $FlowFixMe
        `${type} ${identifier} ${JSON.stringify(data)}\n`,
        () => {},
      );
    }
  }
}

export const identifierRegistry: IdentifierRegistry = new IdentifierRegistry();
