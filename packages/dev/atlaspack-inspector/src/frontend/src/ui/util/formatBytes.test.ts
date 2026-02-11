import {formatBytes} from './formatBytes';

describe('formatBytes', () => {
  it('should format bytes correctly', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1024 * 1024)).toBe('1.00 MB');
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.00 GB');

    // other examples
    expect(formatBytes(1024 * 1024 * 2.55)).toBe('2.55 MB');
  });
});
