import assert from 'assert';
import sinon from 'sinon';

import {
  serialize,
  deserialize,
  registerSerializableClass,
  unregisterSerializableClass,
} from '../src/serializer.mts';

describe('serializer', () => {
  it('should serialize a basic object', () => {
    const serialized = serialize({foo: 2, bar: 3});
    assert(Buffer.isBuffer(serialized));
    const deserialized = deserialize(serialized);
    assert.equal(typeof deserialized, 'object');
    assert.deepEqual(deserialized, {foo: 2, bar: 3});
  });

  it('should serialize an object with multiple references', () => {
    const a = {foo: 2};
    const b = {bar: a, baz: a};
    const res = deserialize(serialize(b));
    assert.deepEqual(res, b);
    assert.equal(res.bar, res.baz);
  });

  it('should serialize a cyclic object', () => {
    const a = {foo: 2, bar: {}};
    a.bar = a;
    const res = deserialize(serialize(a));
    assert.deepEqual(res, a);
    assert.equal(res.bar, res);
    assert.equal(a.bar, a);
  });

  it('should serialize a Map', () => {
    const a = new Map([[2, 3]]);
    const res = deserialize(serialize(a));
    assert(res instanceof Map);
    assert.equal(res.get(2), 3);
  });

  it('should serialize a Set', () => {
    const a = new Set([2, 3]);
    const res = deserialize(serialize(a));
    assert(res instanceof Set);
    assert(res.has(2));
    assert(res.has(3));
  });

  it('should serialize a class', () => {
    class Test {
      x: number;
      constructor(x: number) {
        this.x = x;
      }
    }

    registerSerializableClass('Test', Test);

    const x = new Test(2);
    const res = deserialize(serialize(x));
    assert(res instanceof Test);
    assert.equal(res.x, x.x);

    unregisterSerializableClass('Test', Test);
  });

  it('should serialize a class with a custom serialize method', () => {
    class Test {
      x: number;
      constructor(x: number) {
        this.x = x;
      }

      serialize() {
        return {
          x: this.x,
          serialized: true,
        };
      }
    }

    registerSerializableClass('Test', Test);

    const x = new Test(2);
    const res = deserialize(serialize(x));
    assert(res instanceof Test);
    assert.equal(res.x, x.x);
    // @ts-expect-error no type
    assert.equal(res.serialized, true);

    unregisterSerializableClass('Test', Test);
  });

  it('should serialize a class with a custom deserialize method', () => {
    class Test {
      x: number;
      constructor(x: number) {
        this.x = x;
      }

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      static deserialize(x: any) {
        return {
          deserialized: true,
          value: x,
        };
      }
    }

    registerSerializableClass('Test', Test);

    const x = new Test(2);
    const res = deserialize(serialize(x));
    assert(!(res instanceof Test));
    assert.equal(res.value.x, x.x);
    assert.equal(res.deserialized, true);

    unregisterSerializableClass('Test', Test);
  });

  it('should serialize a class recursively', () => {
    class Foo {
      x: number;
      constructor(x: number) {
        this.x = x;
      }
    }

    class Bar {
      foo: Foo;
      constructor(foo: Foo) {
        this.foo = foo;
      }
    }

    registerSerializableClass('Foo', Foo);
    registerSerializableClass('Bar', Bar);

    const x = new Bar(new Foo(2));
    const res = deserialize(serialize(x));
    assert(res instanceof Bar);
    assert(res.foo instanceof Foo);
    assert.equal(res.foo.x, 2);

    unregisterSerializableClass('Foo', Foo);
    unregisterSerializableClass('Bar', Bar);
  });

  it('should serialize a cyclic class', () => {
    class Foo {
      x?: Foo;
      constructor(x?: Foo) {
        this.x = x;
      }
    }

    registerSerializableClass('Foo', Foo);

    const x = new Foo();
    x.x = x;

    const res = deserialize(serialize(x));
    assert(res instanceof Foo);
    assert(res.x instanceof Foo);
    assert.equal(res.x, res);

    assert.equal(x.x, x);

    unregisterSerializableClass('Foo', Foo);
  });

  it('should copy on write', () => {
    class Foo {
      x: number;
      constructor(x: number) {
        this.x = x;
      }
    }

    registerSerializableClass('Foo', Foo);

    const x = {y: {foo: new Foo(2)}};

    const res = deserialize(serialize(x));
    assert(res.y.foo instanceof Foo);
    assert(x.y.foo instanceof Foo);

    unregisterSerializableClass('Foo', Foo);
  });

  it('should serialize a cyclic class and copy on write', () => {
    class Foo {
      x?: Foo;
      constructor(x?: Foo) {
        this.x = x;
      }
    }

    registerSerializableClass('Foo', Foo);

    const x = new Foo();
    x.x = x;
    const y = {x: {y: x}};

    const res = deserialize(serialize(y));
    assert(res.x.y instanceof Foo);
    assert(res.x.y.x instanceof Foo);
    assert(y.x.y instanceof Foo);
    assert(y.x.y.x instanceof Foo);
    assert.equal(res.x.y.x, res.x.y);

    assert.equal(x.x, x);

    unregisterSerializableClass('Foo', Foo);
  });

  it('should serialize a class inside a Map', () => {
    class Test {
      x: number;
      constructor(x: number) {
        this.x = x;
      }
    }

    registerSerializableClass('Test', Test);

    const x = new Map([[2, new Test(2)]]);
    const res = deserialize(serialize(x));
    assert(res instanceof Map);
    assert(res.get(2) instanceof Test);

    unregisterSerializableClass('Test', Test);
  });

  it('should serialize a class inside a Set', () => {
    class Test {
      x: number;
      constructor(x: number) {
        this.x = x;
      }
    }

    registerSerializableClass('Test', Test);

    const x = new Set([new Test(2)]);
    const res = deserialize(serialize(x));
    assert(res instanceof Set);
    assert(res.values().next().value instanceof Test);

    unregisterSerializableClass('Test', Test);
  });

  describe('raw values', () => {
    class Outer {
      inner: Inner;

      constructor(inner: Inner) {
        this.inner = inner;
      }

      serialize() {
        return {
          $$raw: true,
          inner: this.inner,
        };
      }
    }

    class Inner {
      x: number;

      constructor(x: number) {
        this.x = x;
      }

      static deserialize = sinon.spy();
    }

    beforeEach(() => {
      registerSerializableClass('Outer', Outer);
      registerSerializableClass('Inner', Inner);
    });

    afterEach(() => {
      unregisterSerializableClass('Outer', Outer);
      unregisterSerializableClass('Inner', Inner);
    });

    it('should not recursively serialize raw values', () => {
      const res = deserialize(serialize(new Outer(new Inner(42))));
      assert(res instanceof Outer);
      assert(!(res.inner instanceof Inner));
      // @ts-expect-error type
      assert.equal(res.inner.x, 42);
    });

    it('should not recursively deserialize raw values', () => {
      deserialize(serialize(new Outer(new Inner(42))));
      assert(Inner.deserialize.notCalled);
    });
  });
});
