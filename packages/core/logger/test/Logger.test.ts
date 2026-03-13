import assert from 'assert';
import sinon from 'sinon';
import Logger, {PluginLogger} from '../src/Logger';
import type {LogEvent} from '@atlaspack/types-internal';

const ORIGIN = 'test-plugin';

describe('Logger', () => {
  let onLog: sinon.SinonSpy<[LogEvent]>;
  let logDisposable: {dispose(): void};

  beforeEach(() => {
    onLog = sinon.spy();
    logDisposable = Logger.onLog(onLog);
  });

  afterEach(() => {
    logDisposable.dispose();
  });

  const diagnostic = {message: 'hello', origin: ORIGIN};

  it('emits log diagnostics with info level', () => {
    Logger.log(diagnostic);
    assert(
      onLog.calledWith({type: 'log', level: 'info', diagnostics: [diagnostic]}),
    );
  });

  it('emits info() diagnostics with info level', () => {
    Logger.info(diagnostic);
    assert(
      onLog.calledWith({type: 'log', level: 'info', diagnostics: [diagnostic]}),
    );
  });

  it('emits verbose() diagnostics with verbose level', () => {
    Logger.verbose(diagnostic);
    assert(
      onLog.calledWith({
        type: 'log',
        level: 'verbose',
        diagnostics: [diagnostic],
      }),
    );
  });

  it('emits warn diagnostic with warn level', () => {
    Logger.warn(diagnostic);
    assert(
      onLog.calledWith({type: 'log', level: 'warn', diagnostics: [diagnostic]}),
    );
  });

  it('emits error messages with error level', () => {
    Logger.error(diagnostic);
    assert(
      onLog.calledWith({
        type: 'log',
        level: 'error',
        diagnostics: [diagnostic],
      }),
    );
  });

  it('stamps origin on error() when realOrigin is provided', () => {
    Logger.error({message: 'oops'}, ORIGIN);
    assert(
      onLog.calledWith({
        type: 'log',
        level: 'error',
        diagnostics: [{message: 'oops', origin: ORIGIN}],
      }),
    );
  });

  it('emits progress messages with progress level', () => {
    Logger.progress('update');
    assert(
      onLog.calledWith({type: 'log', level: 'progress', message: 'update'}),
    );
  });
});

describe('PluginLogger', () => {
  let onLog: sinon.SinonSpy<[LogEvent]>;
  let logDisposable: {dispose(): void};

  beforeEach(() => {
    onLog = sinon.spy();
    logDisposable = Logger.onLog(onLog);
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

  it('does not bleed origin between instances', () => {
    new PluginLogger({origin: 'plugin-a'}).info({message: 'from a'});
    new PluginLogger({origin: 'plugin-b'}).info({message: 'from b'});

    assert.equal(onLog.callCount, 2);
    assert.equal(onLog.firstCall.args[0].diagnostics[0].origin, 'plugin-a');
    assert.equal(onLog.secondCall.args[0].diagnostics[0].origin, 'plugin-b');
  });

  it('each method emits exactly once', () => {
    for (const method of ['verbose', 'info', 'log', 'warn', 'error'] as const) {
      onLog.resetHistory();
      new PluginLogger({origin: ORIGIN})[method](diagnostic);
      assert.equal(onLog.callCount, 1, `${method}() should emit exactly once`);
    }
  });
});
