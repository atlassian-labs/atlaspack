import fs from 'fs';

export class IdentifierRegistry {
  #enabled: boolean;
  #createdDirectory: boolean = false;

  constructor(enabled: boolean) {
    this.#enabled = enabled;
  }

  addIdentifier(type: string, identifier: string, data: unknown) {
    if (this.#enabled) {
      if (!this.#createdDirectory) {
        fs.mkdirSync('./.atlaspack', {recursive: true});
        this.#createdDirectory = true;
      }

      fs.appendFile(
        './.atlaspack/atlaspack-identifiers.txt',
        `${type} ${identifier} ${JSON.stringify(data)}\n`,
        () => {},
      );
    }
  }
}

export const identifierRegistry: IdentifierRegistry = new IdentifierRegistry(
  process.env.ATLASPACK_IDENTIFIER_DEBUG === 'true',
);
