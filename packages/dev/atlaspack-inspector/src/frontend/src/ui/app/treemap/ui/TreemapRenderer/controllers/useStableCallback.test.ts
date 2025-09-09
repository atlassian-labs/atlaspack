import {renderHook} from '@testing-library/react';
import {useStableCallback} from './useStableCallback';

describe('useStableCallback', () => {
  it('should return a stable callback', () => {
    const callback1 = jest.fn();
    const {result, rerender} = renderHook(useStableCallback, {
      initialProps: callback1,
    });

    expect(result.current).toBeInstanceOf(Function);

    result.current();
    result.current();
    const previousRef = result.current;
    expect(callback1).toHaveBeenCalledTimes(2);

    const callback2 = jest.fn();
    rerender(callback2);
    expect(result.current).toBe(previousRef);

    result.current();
    result.current();
    expect(callback1).toHaveBeenCalledTimes(2);
    expect(callback2).toHaveBeenCalledTimes(2);
  });
});
