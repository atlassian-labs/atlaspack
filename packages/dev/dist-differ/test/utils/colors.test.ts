import assert from 'assert';
import {getColors} from '../../src/utils/colors';

describe('getColors', () => {
  const originalIsTTY = process.stdout.isTTY;

  afterEach(() => {
    Object.defineProperty(process.stdout, 'isTTY', {
      value: originalIsTTY,
      writable: true,
      configurable: true,
    });
  });

  it('should return color codes when stdout is a TTY', () => {
    Object.defineProperty(process.stdout, 'isTTY', {
      value: true,
      writable: true,
      configurable: true,
    });

    const colors = getColors();

    assert.equal(colors.reset, '\x1b[0m');
    assert.equal(colors.red, '\x1b[31m');
    assert.equal(colors.green, '\x1b[32m');
    assert.equal(colors.yellow, '\x1b[33m');
    assert.equal(colors.cyan, '\x1b[36m');
    assert.equal(colors.dim, '\x1b[2m');
  });

  it('should return empty strings when stdout is not a TTY', () => {
    Object.defineProperty(process.stdout, 'isTTY', {
      value: false,
      writable: true,
      configurable: true,
    });

    const colors = getColors();

    assert.equal(colors.reset, '');
    assert.equal(colors.red, '');
    assert.equal(colors.green, '');
    assert.equal(colors.yellow, '');
    assert.equal(colors.cyan, '');
    assert.equal(colors.dim, '');
  });

  it('should return empty strings when stdout.isTTY is undefined', () => {
    Object.defineProperty(process.stdout, 'isTTY', {
      value: undefined,
      writable: true,
      configurable: true,
    });

    const colors = getColors();

    assert.equal(colors.reset, '');
    assert.equal(colors.red, '');
    assert.equal(colors.green, '');
    assert.equal(colors.yellow, '');
    assert.equal(colors.cyan, '');
    assert.equal(colors.dim, '');
  });
});
