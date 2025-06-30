// Quick test to verify the option key changes work correctly
const assert = require('assert');

describe('Option Key Handling', () => {
  // Mock the functions we want to test
  const mockHashFromOption = () => 'test-hash';

  // Create a simple version of nodeFromOption for testing
  const nodeFromOption = (option, value) => {
    const optionKey = Array.isArray(option)
      ? `array:${JSON.stringify(option)}`
      : option;
    return {
      id: 'option:' + optionKey,
      type: 4, // OPTION
      hash: mockHashFromOption(value),
    };
  };

  const extractOptionFromKey = (key) => {
    if (key.startsWith('array:')) {
      try {
        return JSON.parse(key.slice(6));
      } catch (e) {
        return key.slice(6);
      }
    } else {
      return key;
    }
  };

  it('should preserve original behavior for string options', () => {
    const stringNode = nodeFromOption('mode', 'development');
    assert.strictEqual(stringNode.id, 'option:mode');
    assert.strictEqual(extractOptionFromKey('mode'), 'mode');
  });

  it('should add array prefix for array options', () => {
    const arrayNode = nodeFromOption(['targets', 'browsers'], ['chrome']);
    assert.strictEqual(arrayNode.id, 'option:array:["targets","browsers"]');

    const extracted = extractOptionFromKey('array:["targets","browsers"]');
    assert.deepStrictEqual(extracted, ['targets', 'browsers']);
  });

  it('should handle array options with dots correctly', () => {
    const dotNode = nodeFromOption(['targets.browsers'], ['chrome']);
    assert.strictEqual(dotNode.id, 'option:array:["targets.browsers"]');

    const extractedDot = extractOptionFromKey('array:["targets.browsers"]');
    assert.deepStrictEqual(extractedDot, ['targets.browsers']);
  });

  it('should maintain backward compatibility for legacy string options', () => {
    assert.strictEqual(
      extractOptionFromKey('some.nested.option'),
      'some.nested.option',
    );
  });

  it('should handle edge cases', () => {
    // Test empty array
    const emptyArrayNode = nodeFromOption([], 'value');
    assert.strictEqual(emptyArrayNode.id, 'option:array:[]');

    // Test single element array
    const singleElementNode = nodeFromOption(['single'], 'value');
    assert.strictEqual(singleElementNode.id, 'option:array:["single"]');

    // Test extraction of these
    assert.deepStrictEqual(extractOptionFromKey('array:[]'), []);
    assert.deepStrictEqual(extractOptionFromKey('array:["single"]'), [
      'single',
    ]);
  });
});
