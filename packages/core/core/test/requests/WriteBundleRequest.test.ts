import assert from 'assert';
import SourceMap from '@atlaspack/source-map';
import {
  applyReplacementsToSourceMap,
  type HashRefReplacement,
} from '../../src/requests/WriteBundleRequest';

const HASH_REF = 'HASH_REF_0123456789abcdef'; // 25 chars
const HASH_REPLACEMENT = 'a1b2c3d4'; // 8 chars
const HASH_REF_LEN = HASH_REF.length; // 25
const REPLACEMENT_LEN = HASH_REPLACEMENT.length; // 8

/**
 * Build a single-line code string with HASH_REF placeholders at known positions.
 * Returns original code, replaced code, correct replacement coordinates, and
 * identifier position tracking in both coordinate spaces.
 */
function buildCodeWithHashRefs(
  segments: Array<{type: 'code'; text: string} | {type: 'hashref'}>,
) {
  let code = '';
  const hashPositions: Array<{column: number}> = [];
  const identifierPositions = new Map<string, number>();

  for (const seg of segments) {
    if (seg.type === 'hashref') {
      hashPositions.push({column: code.length});
      code += HASH_REF;
    } else {
      const regex = /[A-Z][A-Z_0-9]{3,}/g;
      let match;
      while ((match = regex.exec(seg.text)) !== null) {
        identifierPositions.set(match[0], code.length + match.index);
      }
      code += seg.text;
    }
  }

  // Build replaced code and compute CORRECT replacement coordinates
  let replacedCode = '';
  const correctReplacements: HashRefReplacement[] = [];
  let srcIdx = 0;
  for (const hp of hashPositions) {
    replacedCode += code.slice(srcIdx, hp.column);
    correctReplacements.push({
      line: 0,
      column: replacedCode.length,
      originalLength: HASH_REF_LEN,
      newLength: REPLACEMENT_LEN,
    });
    replacedCode += HASH_REPLACEMENT;
    srcIdx = hp.column + HASH_REF_LEN;
  }
  replacedCode += code.slice(srcIdx);

  // Track where identifiers end up in the replaced code
  const replacedIdentifierPositions = new Map<string, number>();
  for (const [name] of identifierPositions) {
    const idx = replacedCode.indexOf(name);
    if (idx >= 0) replacedIdentifierPositions.set(name, idx);
  }

  return {
    originalCode: code,
    replacedCode,
    correctReplacements,
    identifierPositions,
    replacedIdentifierPositions,
  };
}

describe('applyReplacementsToSourceMap', () => {
  describe('with correct replacement coordinates', () => {
    it('should correctly adjust a single HASH_REF replacement', () => {
      const {
        correctReplacements,
        identifierPositions,
        replacedIdentifierPositions,
      } = buildCodeWithHashRefs([
        {type: 'hashref'},
        {type: 'code', text: ';var x=SOME_IDENT;'},
      ]);

      const origCol = identifierPositions.get('SOME_IDENT')!;
      const expectedCol = replacedIdentifierPositions.get('SOME_IDENT')!;

      const sm = new SourceMap('/');
      sm.addIndexedMapping({
        generated: {line: 1, column: origCol},
        original: {line: 10, column: 5},
        source: 'test.js',
      });

      applyReplacementsToSourceMap(sm, correctReplacements);

      const mappings = sm.getMap().mappings;
      const mapping = mappings.find(
        (m) => m.original?.line === 10 && m.original?.column === 5,
      );
      assert.ok(mapping, 'Mapping should exist');
      assert.strictEqual(mapping!.generated.column, expectedCol);
    });

    it('should correctly adjust multiple HASH_REF replacements on the same line', () => {
      const {
        correctReplacements,
        identifierPositions,
        replacedIdentifierPositions,
      } = buildCodeWithHashRefs([
        {type: 'hashref'},
        {type: 'code', text: ';var a=IDENT_ALPHA;require("'},
        {type: 'hashref'},
        {type: 'code', text: '");var b=IDENT_BETA;require("'},
        {type: 'hashref'},
        {type: 'code', text: '");var c=IDENT_GAMMA;'},
      ]);

      const sm = new SourceMap('/');
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get('IDENT_ALPHA')!},
        original: {line: 10, column: 0},
        source: 'test.js',
      });
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get('IDENT_BETA')!},
        original: {line: 20, column: 0},
        source: 'test.js',
      });
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get('IDENT_GAMMA')!},
        original: {line: 30, column: 0},
        source: 'test.js',
      });

      applyReplacementsToSourceMap(sm, correctReplacements);

      const mappings = sm.getMap().mappings;
      for (const [name, origLine] of [
        ['IDENT_ALPHA', 10],
        ['IDENT_BETA', 20],
        ['IDENT_GAMMA', 30],
      ] as const) {
        const mapping = mappings.find((m) => m.original?.line === origLine);
        const expectedCol = replacedIdentifierPositions.get(name)!;
        assert.ok(mapping, `${name} mapping should exist`);
        assert.strictEqual(
          mapping!.generated.column,
          expectedCol,
          `${name}: expected col ${expectedCol}, got ${mapping!.generated.column}`,
        );
      }
    });

    it('should handle 10 replacements on the same line', () => {
      const segments: Array<{type: 'code'; text: string} | {type: 'hashref'}> =
        [];
      for (let i = 0; i < 10; i++) {
        segments.push({type: 'hashref'});
        segments.push({
          type: 'code',
          text: `;var x${i}=TARGET_${String(i).padStart(2, '0')};require("`,
        });
      }
      segments.push({type: 'code', text: '");'});

      const {
        correctReplacements,
        identifierPositions,
        replacedIdentifierPositions,
      } = buildCodeWithHashRefs(segments);

      const sm = new SourceMap('/');
      for (let i = 0; i < 10; i++) {
        const name = `TARGET_${String(i).padStart(2, '0')}`;
        sm.addIndexedMapping({
          generated: {line: 1, column: identifierPositions.get(name)!},
          original: {line: (i + 1) * 10, column: 0},
          source: 'test.js',
        });
      }

      applyReplacementsToSourceMap(sm, correctReplacements);

      const mappings = sm.getMap().mappings;
      for (let i = 0; i < 10; i++) {
        const name = `TARGET_${String(i).padStart(2, '0')}`;
        const mapping = mappings.find((m) => m.original?.line === (i + 1) * 10);
        const expectedCol = replacedIdentifierPositions.get(name)!;
        assert.ok(mapping, `${name} mapping should exist`);
        assert.strictEqual(
          mapping!.generated.column,
          expectedCol,
          `${name}: expected col ${expectedCol}, got ${mapping!.generated.column}`,
        );
      }
    });

    it('should not affect mappings before the first HASH_REF', () => {
      const {
        correctReplacements,
        identifierPositions,
        replacedIdentifierPositions,
      } = buildCodeWithHashRefs([
        {type: 'code', text: 'var BEFORE_HASH=1;'},
        {type: 'hashref'},
        {type: 'code', text: ';var AFTER_HASH=2;'},
      ]);

      const sm = new SourceMap('/');
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get('BEFORE_HASH')!},
        original: {line: 1, column: 0},
        source: 'test.js',
      });
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get('AFTER_HASH')!},
        original: {line: 2, column: 0},
        source: 'test.js',
      });

      applyReplacementsToSourceMap(sm, correctReplacements);

      const mappings = sm.getMap().mappings;
      const beforeMapping = mappings.find((m) => m.original?.line === 1);
      assert.ok(beforeMapping);
      assert.strictEqual(
        beforeMapping!.generated.column,
        replacedIdentifierPositions.get('BEFORE_HASH'),
      );

      const afterMapping = mappings.find((m) => m.original?.line === 2);
      assert.ok(afterMapping);
      assert.strictEqual(
        afterMapping!.generated.column,
        replacedIdentifierPositions.get('AFTER_HASH'),
      );
    });
  });
});
