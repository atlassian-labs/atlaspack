import sinon from 'sinon';
import {makeConfigProxy} from '../../src/public/Config';
import assert from 'assert';

describe('makeConfigProxy', () => {
  it('tracks reads to nested fields', () => {
    const onRead = sinon.spy();
    const target = {a: {b: {c: 'd'}}} as const;
    const config = makeConfigProxy(onRead, target);
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    config.a.b.c;
    assert.ok(onRead.calledWith(['a', 'b', 'c']));
    assert.ok(onRead.calledOnce);
  });

  it('works for reading package.json dependencies', () => {
    const packageJson = {
      dependencies: {
        react: '18.2.0',
      },
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);
    assert.equal(config.dependencies.react, '18.2.0');
    assert.equal(config.dependencies.preact, undefined);
    assert.ok(onRead.calledWith(['dependencies', 'react']));
    assert.ok(onRead.calledWith(['dependencies', 'preact']));
    assert.equal(onRead.callCount, 2);
  });

  it('will track reads for any missing or null keys', () => {
    const packageJson = {
      dependencies: {
        react: '18.2.0',
      },
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);

    assert.equal(config.alias?.react, undefined);
    assert.ok(onRead.calledWith(['alias']));
    assert.equal(onRead.callCount, 1);
  });

  it('iterating over keys works normally and will register a read for the key being enumerated', () => {
    const packageJson = {
      nested: {
        dependencies: {
          react: '18.2.0',
          'react-dom': '18.2.0',
          'react-router': '6.14.2',
        },
      },
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);
    assert.equal(Object.keys(config.nested.dependencies).length, 3);

    assert.ok(onRead.calledWith(['nested', 'dependencies']));
  });

  it('if a key has an array value we will track a read for that key', () => {
    const packageJson = {
      scripts: ['build', 'test'],
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);
    assert.equal(config.scripts[0], 'build');
    assert.equal(onRead.callCount, 1);
    assert.ok(onRead.calledWith(['scripts']));
  });

  it('if a key array value is iterated over we will track a read for that key', () => {
    const packageJson = {
      scripts: ['build', 'test'],
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);
    let scriptCount = 0;
    // eslint-disable-next-line no-unused-vars
    for (const _script of config.scripts) {
      scriptCount += 1;
    }
    assert.equal(scriptCount, 2);
    assert.ok(onRead.calledWith(['scripts']));
    assert.equal(onRead.callCount, 1);
  });

  it('if a key array value length is verified we will track a read for that key', () => {
    const packageJson = {
      scripts: ['build', 'test'],
    } as const;

    const onRead = sinon.spy();
    const config = makeConfigProxy(onRead, packageJson);
    assert.equal(config.scripts.length, 2);
    assert.ok(onRead.calledWith(['scripts']));
    assert.equal(onRead.callCount, 1);
  });
});
