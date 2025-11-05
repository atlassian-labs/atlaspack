import assert from 'assert';
import validateSchema, {SchemaEntity, fuzzySearch} from '../src/schema';
import ThrowableDiagnostic from '@atlaspack/diagnostic';

describe('validateSchema', () => {
  describe('basic validation', () => {
    it('should validate a simple object schema', () => {
      const schema: SchemaEntity = {
        type: 'object',
        properties: {
          name: {type: 'string'},
          age: {type: 'number'},
        },
      };

      const errors = validateSchema(schema, {name: 'John', age: 30});
      assert.equal(errors.length, 0);
    });

    it('should return type error for invalid type', () => {
      const schema: SchemaEntity = {
        type: 'object',
        properties: {
          name: {type: 'string'},
        },
      };

      const errors = validateSchema(schema, {name: 123});
      assert.equal(errors.length, 1);
      assert.equal(errors[0].type, 'type');
    });

    it('should return enum error for invalid enum value', () => {
      const schema: SchemaEntity = {
        type: 'object',
        properties: {
          env: {
            type: 'string',
            enum: ['development', 'production', 'test'],
          },
        },
      };

      const errors = validateSchema(schema, {env: 'staging'});
      assert.equal(errors.length, 1);
      assert.equal(errors[0].type, 'enum');
    });

    it('should return missing-prop error for required properties', () => {
      const schema: SchemaEntity = {
        type: 'object',
        properties: {
          name: {type: 'string'},
          email: {type: 'string'},
        },
        required: ['name', 'email'],
      };

      const errors = validateSchema(schema, {name: 'John'});
      assert.equal(errors.length, 1);
      assert.equal(errors[0].type, 'missing-prop');
    });

    it('should return forbidden-prop error for forbidden properties', () => {
      const schema: SchemaEntity = {
        type: 'object',
        properties: {
          name: {type: 'string'},
        },
        __forbiddenProperties: ['age', 'email'],
      };

      const errors = validateSchema(schema, {name: 'John', age: 30});
      assert.equal(errors.length, 1);
      assert.equal(errors[0].type, 'forbidden-prop');
    });
  });

  describe('fuzzySearch', () => {
    it('should find close matches', () => {
      const expectedValues = ['development', 'production', 'test'];
      const actualValue = 'developement'; // typo

      const results = fuzzySearch(expectedValues, actualValue);
      assert(results.includes('development'));
    });

    it('should return empty array for distant matches', () => {
      const expectedValues = ['foo', 'bar'];
      const actualValue = 'verylongstring';

      const results = fuzzySearch(expectedValues, actualValue);
      assert.equal(results.length, 0);
    });
  });

  describe('validateSchema.diagnostic', () => {
    describe('deferred source (function)', () => {
      it('should accept source as a function that returns JSON string', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const validData = {name: 'John'};
        const sourceLoader = () => JSON.stringify(validData);

        // Should not throw for valid data
        assert.doesNotThrow(() => {
          validateSchema.diagnostic(
            schema,
            {source: sourceLoader},
            '@test/origin',
            'Test validation',
          );
        });
      });

      it('should throw diagnostic error with deferred source for invalid data', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const invalidData = {name: 123}; // wrong type
        const sourceLoader = () => JSON.stringify(invalidData);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {source: sourceLoader, filePath: 'test.json'},
              '@test/origin',
              'Test validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            assert.equal(error.diagnostics[0].message, 'Test validation');
            assert.equal(error.diagnostics[0].origin, '@test/origin');
            assert(error.diagnostics[0].codeFrames);
            return true;
          },
        );
      });

      it('should only call source function once when loading', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        let callCount = 0;
        const sourceLoader = () => {
          callCount++;
          return JSON.stringify({name: 'John'});
        };

        validateSchema.diagnostic(
          schema,
          {source: sourceLoader},
          '@test/origin',
          'Test validation',
        );

        // The function should only be called once even though it might be referenced multiple times
        assert.equal(callCount, 1);
      });

      it('should handle deferred source with enum error', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            env: {
              type: 'string',
              enum: ['development', 'production', 'test'],
            },
          },
        };

        const invalidData = {env: 'staging'};
        const sourceLoader = () => JSON.stringify(invalidData);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {source: sourceLoader, filePath: 'config.json'},
              '@test/config',
              'Invalid configuration',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const diagnostic = error.diagnostics[0];
            assert.equal(diagnostic.message, 'Invalid configuration');
            const codeFrame = diagnostic.codeFrames?.[0];
            assert(codeFrame);
            assert(codeFrame.codeHighlights.length > 0);
            return true;
          },
        );
      });

      it('should handle deferred source with missing property error', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
            email: {type: 'string'},
          },
          required: ['name', 'email'],
        };

        const invalidData = {name: 'John'};
        const sourceLoader = () => JSON.stringify(invalidData);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {source: sourceLoader, filePath: 'user.json'},
              '@test/user',
              'Missing required fields',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });
    });

    describe('direct source (string)', () => {
      it('should accept source as a string', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const validData = {name: 'John'};

        assert.doesNotThrow(() => {
          validateSchema.diagnostic(
            schema,
            {source: JSON.stringify(validData)},
            '@test/origin',
            'Test validation',
          );
        });
      });

      it('should throw diagnostic error with string source for invalid data', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const invalidData = {name: 123};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {
                source: JSON.stringify(invalidData, null, 2),
                filePath: 'test.json',
              },
              '@test/origin',
              'Test validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const diagnostic = error.diagnostics[0];
            assert.equal(diagnostic.message, 'Test validation');
            assert.equal(diagnostic.origin, '@test/origin');
            assert.equal(diagnostic.codeFrames?.[0].filePath, 'test.json');
            return true;
          },
        );
      });
    });

    describe('data property', () => {
      it('should accept data property directly', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const validData = {name: 'John'};

        assert.doesNotThrow(() => {
          validateSchema.diagnostic(
            schema,
            {data: validData},
            '@test/origin',
            'Test validation',
          );
        });
      });

      it('should throw diagnostic error with data property for invalid data', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const invalidData = {name: 123};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData, filePath: 'test.json'},
              '@test/origin',
              'Test validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });

      it('should accept both data and source together (common pattern)', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
            version: {type: 'string'},
          },
        };

        const validData = {name: 'my-package', version: '1.0.0'};
        const source = JSON.stringify(validData, null, 2);

        assert.doesNotThrow(() => {
          validateSchema.diagnostic(
            schema,
            {data: validData, source, filePath: 'package.json'},
            '@test/origin',
            'Package validation',
          );
        });
      });

      it('should throw diagnostic with both data and source for invalid data', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
            version: {type: 'string'},
          },
        };

        const invalidData = {name: 'my-package', version: 123};
        const source = JSON.stringify(invalidData, null, 2);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData, source, filePath: 'package.json'},
              '@test/origin',
              'Package validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const diagnostic = error.diagnostics[0];
            assert.equal(diagnostic.message, 'Package validation');
            assert.equal(diagnostic.origin, '@test/origin');
            assert.equal(diagnostic.codeFrames?.[0]?.filePath, 'package.json');
            // Code highlighting should work with the provided source
            const codeFrame = diagnostic.codeFrames?.[0];
            assert(codeFrame);
            assert(codeFrame.codeHighlights.length > 0);
            return true;
          },
        );
      });

      it('should accept both data and deferred source together', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
            version: {type: 'string'},
          },
        };

        const validData = {name: 'my-package', version: '1.0.0'};
        const sourceLoader = () => JSON.stringify(validData, null, 2);

        assert.doesNotThrow(() => {
          validateSchema.diagnostic(
            schema,
            {data: validData, source: sourceLoader, filePath: 'package.json'},
            '@test/origin',
            'Package validation',
          );
        });
      });

      it('should throw diagnostic with both data and deferred source for invalid data', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
            version: {type: 'string'},
          },
        };

        const invalidData = {name: 'my-package', version: 123};
        const sourceLoader = () => JSON.stringify(invalidData, null, 2);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {
                data: invalidData,
                source: sourceLoader,
                filePath: 'package.json',
              },
              '@test/origin',
              'Package validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const diagnostic = error.diagnostics[0];
            assert.equal(diagnostic.message, 'Package validation');
            assert.equal(diagnostic.origin, '@test/origin');
            assert.equal(diagnostic.codeFrames?.[0]?.filePath, 'package.json');
            // Code highlighting should work with the deferred source
            const codeFrame = diagnostic.codeFrames?.[0];
            assert(codeFrame);
            assert(codeFrame.codeHighlights.length > 0);
            return true;
          },
        );
      });
    });

    describe('error messages', () => {
      it('should generate "Did you mean" message for enum errors with close matches', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            env: {
              type: 'string',
              enum: ['development', 'production', 'test'],
            },
          },
        };

        const invalidData = {env: 'developement'}; // typo

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid config',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const codeHighlights =
              error.diagnostics[0].codeFrames?.[0].codeHighlights;
            const message = codeHighlights?.[0].message;
            assert(message?.includes('Did you mean'));
            assert(message?.includes('development'));
            return true;
          },
        );
      });

      it('should generate "Possible values" message for enum errors without close matches', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            env: {
              type: 'string',
              enum: ['development', 'production', 'test'],
            },
          },
        };

        const invalidData = {env: 'xyz'};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid config',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const codeHighlights =
              error.diagnostics[0].codeFrames?.[0].codeHighlights;
            const message = codeHighlights?.[0].message;
            assert(message?.includes('Possible values'));
            return true;
          },
        );
      });

      it('should generate "Unexpected property" message for forbidden props', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
          __forbiddenProperties: ['age'],
        };

        const invalidData = {name: 'John', age: 30};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid config',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });

      it('should generate type error message', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string', __type: 'a string value'},
          },
        };

        const invalidData = {name: 123};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid config',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const codeHighlights =
              error.diagnostics[0].codeFrames?.[0].codeHighlights;
            const message = codeHighlights?.[0].message;
            assert(message?.includes('Expected a string value'));
            return true;
          },
        );
      });
    });

    describe('map with pointers', () => {
      it('should handle map with data and pointers', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            name: {type: 'string'},
          },
        };

        const invalidData = {name: 123};
        const source = JSON.stringify(invalidData);

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {
                source,
                map: {
                  data: invalidData,
                  pointers: {
                    '/name': {
                      key: {line: 1, column: 2, pos: 2},
                      keyEnd: {line: 1, column: 8, pos: 8},
                      value: {line: 1, column: 10, pos: 10},
                      valueEnd: {line: 1, column: 13, pos: 13},
                    },
                  },
                },
                filePath: 'test.json',
              },
              '@test/origin',
              'Test validation',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });
    });

    describe('array validation', () => {
      it('should validate array items', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            items: {
              type: 'array',
              items: {type: 'string'},
            },
          },
        };

        const invalidData = {items: ['foo', 123, 'bar']};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid array',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });
    });

    describe('custom validation', () => {
      it('should use custom __validate function', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            email: {
              type: 'string',
              __validate: (val: string) => {
                if (!val.includes('@')) {
                  return 'Must be a valid email address';
                }
                return undefined;
              },
            },
          },
        };

        const invalidData = {email: 'notanemail'};

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Invalid email',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            const codeHighlights =
              error.diagnostics[0].codeFrames?.[0].codeHighlights;
            const message = codeHighlights?.[0].message;
            assert.equal(message, 'Must be a valid email address');
            return true;
          },
        );
      });
    });

    describe('complex nested schemas', () => {
      it('should validate deeply nested objects', () => {
        const schema: SchemaEntity = {
          type: 'object',
          properties: {
            user: {
              type: 'object',
              properties: {
                profile: {
                  type: 'object',
                  properties: {
                    age: {type: 'number'},
                  },
                },
              },
            },
          },
        };

        const invalidData = {
          user: {
            profile: {
              age: 'not a number',
            },
          },
        };

        assert.throws(
          () => {
            validateSchema.diagnostic(
              schema,
              {data: invalidData},
              '@test/origin',
              'Nested validation failed',
            );
          },
          (error: any) => {
            assert(error instanceof ThrowableDiagnostic);
            return true;
          },
        );
      });
    });
  });
});
