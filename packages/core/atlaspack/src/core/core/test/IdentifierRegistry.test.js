// @flow strict-local

import assert from 'assert';
import fs from 'fs';
import sinon from 'sinon';
import {IdentifierRegistry} from '../IdentifierRegistry';

describe('IdentifierRegistry', () => {
  let mkdirSyncStub;
  let fsStub;

  beforeEach(() => {
    mkdirSyncStub = sinon.stub(fs, 'mkdirSync');
    fsStub = sinon.stub(fs, 'appendFile');
  });

  afterEach(() => {
    mkdirSyncStub.restore();
    fsStub.restore();
  });

  it("does nothing if the identifers flag isn't set", () => {
    const identifierRegistry = new IdentifierRegistry(false);
    identifierRegistry.addIdentifier('type', 'identifier', 'data');
    assert(!mkdirSyncStub.called);
    assert(!fsStub.called);
  });

  it('appends the identifier to the file', () => {
    const identifierRegistry = new IdentifierRegistry(true);
    identifierRegistry.addIdentifier('type', 'identifier', 'data');
    assert(
      fsStub.calledWith(
        './.atlaspack/atlaspack-identifiers.txt',
        'type identifier "data"\n',
        sinon.match.func,
      ),
    );
  });

  it('only creates the directory once', () => {
    const identifierRegistry = new IdentifierRegistry(true);
    identifierRegistry.addIdentifier('type', 'identifier', '1');
    identifierRegistry.addIdentifier('type', 'identifier', '2');

    assert(mkdirSyncStub.calledOnce);

    assert(mkdirSyncStub.calledWith('./.atlaspack', {recursive: true}));
    assert(
      fsStub.calledWith(
        './.atlaspack/atlaspack-identifiers.txt',
        'type identifier "1"\n',
        sinon.match.func,
      ),
    );
    assert(
      fsStub.calledWith(
        './.atlaspack/atlaspack-identifiers.txt',
        'type identifier "2"\n',
        sinon.match.func,
      ),
    );
  });
});
