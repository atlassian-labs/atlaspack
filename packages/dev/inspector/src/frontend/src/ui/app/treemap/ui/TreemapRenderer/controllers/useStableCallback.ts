import {useCallback, useEffect, useRef} from 'react';

/**
 * Like `useCallback` but the callback never changes even if the fn captures change.
 */
export function useStableCallback(fn: (...args: any[]) => void) {
  const ref = useRef(fn);

  useEffect(() => {
    ref.current = fn;
  }, [fn]);

  return useCallback((...args: any[]) => ref.current(...args), []);
}
