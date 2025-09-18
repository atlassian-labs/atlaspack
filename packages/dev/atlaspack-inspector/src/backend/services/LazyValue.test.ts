import {LazyValue} from './LazyValue';

describe('LazyValue', () => {
  it('should return the value by calling the factory function once', () => {
    const mock = jest.fn().mockImplementation(() => 'test');
    const lazyValue = new LazyValue(mock);

    expect(lazyValue.get()).toBe('test');
    expect(lazyValue.get()).toBe('test');

    expect(mock).toHaveBeenCalledTimes(1);
  });
});
