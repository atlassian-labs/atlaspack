import assert from 'assert';
import Disposable from '../src/Disposable';
import {AlreadyDisposedError} from '../src/errors';

describe('Disposable', () => {
  it('can wrap an IDisposable', () => {
    let disposed: boolean | undefined;

    new Disposable({
      dispose() {
        disposed = true;
      },
    }).dispose();
    assert.equal(disposed, true);
  });

  it('can wrap a function to dispose', () => {
    let disposed: boolean | undefined;
    new Disposable(() => {
      disposed = true;
    }).dispose();
    assert.equal(disposed, true);
  });

  it('can wrap many disposable-likes', () => {
    let disposed1: boolean | undefined;
    let disposed2: boolean | undefined;

    new Disposable(
      {
        dispose() {
          disposed1 = true;
        },
      },
      () => {
        disposed2 = true;
      },
    ).dispose();
    assert.equal(disposed1, true);
    assert.equal(disposed2, true);
  });

  it('can add disposables after construction', () => {
    let disposed1: boolean | undefined;
    let disposed2: boolean | undefined;
    let disposed3: boolean | undefined;
    let disposed4: boolean | undefined;

    const disposable = new Disposable(
      {
        dispose() {
          disposed1 = true;
        },
      },
      () => {
        disposed2 = true;
      },
    );

    disposable.add(
      () => {
        disposed3 = true;
      },
      {
        dispose() {
          disposed4 = true;
        },
      },
    );

    assert.notEqual(disposed1, true);
    assert.notEqual(disposed2, true);
    assert.notEqual(disposed3, true);
    assert.notEqual(disposed4, true);

    disposable.dispose();

    assert.equal(disposed1, true);
    assert.equal(disposed2, true);
    assert.equal(disposed3, true);
    assert.equal(disposed4, true);
  });

  it(
    'does not dispose inner disposables more than once,' +
      ' and does not throw on subsequent disposals',
    () => {
      let disposed: boolean | undefined;
      const disposable = new Disposable(() => {
        if (disposed) {
          // $FlowFixMe
          assert.fail();
        }
        disposed = true;
      });

      disposable.dispose();
      disposable.dispose();
    },
  );

  it('throws if `add` is called after it has been disposed', () => {
    const disposable = new Disposable();
    disposable.dispose();
    assert.throws(() => {
      disposable.add(() => {});
    }, AlreadyDisposedError);
  });

  it('can be checked for disposal state', () => {
    const disposable = new Disposable();
    assert.equal(disposable.disposed, false);
    disposable.dispose();
    assert.equal(disposable.disposed, true);
  });
});
