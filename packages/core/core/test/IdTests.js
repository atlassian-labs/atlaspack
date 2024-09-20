// @flow strict-local

import expect from 'expect';
import {createEnvironment} from '../src/Environment';

describe('createEnvironment', function () {
  it('returns a stable hash', () => {
    const environment = createEnvironment({});
    expect(environment.id).toEqual('c242f987e3544367');
  });
});
