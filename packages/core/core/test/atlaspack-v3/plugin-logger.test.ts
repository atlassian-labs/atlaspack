import assert from 'assert';
import sinon from 'sinon';
import logger from '@atlaspack/logger';
import {PluginLogger} from '../../src/atlaspack-v3/worker/compat/plugin-logger';

const ORIGIN = 'test-plugin';

describe('PluginLogger (v3 compat)', () => {
  let onLog: sinon.SinonSpy;
  let logDisposable: {dispose(): void};

  beforeEach(() => {
    onLog = sinon.spy();
    logDisposable = logger.onLog(onLog);
  });

  afterEach(() => {
    logDisposable.dispose();
  });

  const diagnostic = {message: 'test message'};
  const diagnosticArray = [{message: 'first'}, {message: 'second'}];

  describe('verbose()', () => {
    it('forwards a single diagnostic at verbose level with origin', () => {
      new PluginLogger({origin: ORIGIN}).verbose(diagnostic);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'verbose',
          diagnostics: [{...diagnostic, origin: ORIGIN}],
        }),
      );
    });

    it('stamps origin on every diagnostic in an array', () => {
      new PluginLogger({origin: ORIGIN}).verbose(diagnosticArray);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'verbose',
          diagnostics: diagnosticArray.map((d) => ({...d, origin: ORIGIN})),
        }),
      );
    });
  });

  describe('info()', () => {
    it('forwards a single diagnostic at info level with origin', () => {
      new PluginLogger({origin: ORIGIN}).info(diagnostic);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'info',
          diagnostics: [{...diagnostic, origin: ORIGIN}],
        }),
      );
    });

    it('stamps origin on every diagnostic in an array', () => {
      new PluginLogger({origin: ORIGIN}).info(diagnosticArray);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'info',
          diagnostics: diagnosticArray.map((d) => ({...d, origin: ORIGIN})),
        }),
      );
    });
  });

  describe('log()', () => {
    it('forwards a single diagnostic at info level with origin', () => {
      new PluginLogger({origin: ORIGIN}).log(diagnostic);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'info',
          diagnostics: [{...diagnostic, origin: ORIGIN}],
        }),
      );
    });

    it('stamps origin on every diagnostic in an array', () => {
      new PluginLogger({origin: ORIGIN}).log(diagnosticArray);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'info',
          diagnostics: diagnosticArray.map((d) => ({...d, origin: ORIGIN})),
        }),
      );
    });
  });

  describe('warn()', () => {
    it('forwards a single diagnostic at warn level with origin', () => {
      new PluginLogger({origin: ORIGIN}).warn(diagnostic);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'warn',
          diagnostics: [{...diagnostic, origin: ORIGIN}],
        }),
      );
    });

    it('stamps origin on every diagnostic in an array', () => {
      new PluginLogger({origin: ORIGIN}).warn(diagnosticArray);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'warn',
          diagnostics: diagnosticArray.map((d) => ({...d, origin: ORIGIN})),
        }),
      );
    });
  });

  describe('error()', () => {
    it('forwards a diagnostic at error level with origin', () => {
      new PluginLogger({origin: ORIGIN}).error(diagnostic);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'error',
          diagnostics: [{...diagnostic, origin: ORIGIN}],
        }),
      );
    });

    it('stamps origin on every diagnostic in an array', () => {
      new PluginLogger({origin: ORIGIN}).error(diagnosticArray);
      assert(
        onLog.calledWith({
          type: 'log',
          level: 'error',
          diagnostics: diagnosticArray.map((d) => ({...d, origin: ORIGIN})),
        }),
      );
    });

    it('accepts a plain Error object and stamps origin', () => {
      const err = new Error('something went wrong');
      new PluginLogger({origin: ORIGIN}).error(err);
      assert(onLog.calledOnce);
      const event = onLog.firstCall.args[0];
      assert.equal(event.type, 'log');
      assert.equal(event.level, 'error');
      assert.ok(Array.isArray(event.diagnostics));
      assert.equal(event.diagnostics[0].message, 'something went wrong');
      assert.equal(event.diagnostics[0].origin, ORIGIN);
    });
  });

  describe('origin does not bleed between instances', () => {
    it('uses each instance origin independently', () => {
      new PluginLogger({origin: 'plugin-a'}).info({message: 'from a'});
      new PluginLogger({origin: 'plugin-b'}).info({message: 'from b'});

      assert.equal(onLog.callCount, 2);
      assert.equal(onLog.firstCall.args[0].diagnostics[0].origin, 'plugin-a');
      assert.equal(onLog.secondCall.args[0].diagnostics[0].origin, 'plugin-b');
    });
  });

  describe('each method emits exactly once', () => {
    it('verbose emits once', () => {
      new PluginLogger({origin: ORIGIN}).verbose(diagnostic);
      assert.equal(onLog.callCount, 1);
    });

    it('info emits once', () => {
      new PluginLogger({origin: ORIGIN}).info(diagnostic);
      assert.equal(onLog.callCount, 1);
    });

    it('log emits once', () => {
      new PluginLogger({origin: ORIGIN}).log(diagnostic);
      assert.equal(onLog.callCount, 1);
    });

    it('warn emits once', () => {
      new PluginLogger({origin: ORIGIN}).warn(diagnostic);
      assert.equal(onLog.callCount, 1);
    });

    it('error emits once', () => {
      new PluginLogger({origin: ORIGIN}).error(diagnostic);
      assert.equal(onLog.callCount, 1);
    });
  });
});
