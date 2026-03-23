import assert from 'assert';
import {Readable} from 'stream';
import SourceMap from '@atlaspack/source-map';
import {
  applyReplacementsToSourceMap,
  applyReplacementsToVLQMappings,
  SourceMapHashRefRewriteStream,
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

/**
 * Drains a Readable stream into a single Buffer.
 */
function streamToBuffer(stream: Readable): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    stream.on('data', (chunk: Buffer) => chunks.push(chunk));
    stream.on('end', () => resolve(Buffer.concat(chunks)));
    stream.on('error', reject);
  });
}

/**
 * Cross-checks applyReplacementsToVLQMappings against the native
 * applyReplacementsToSourceMap by building a SourceMap, running both
 * implementations, and asserting identical VLQ output.
 */
function crossCheck(sm: SourceMap, replacements: HashRefReplacement[]): void {
  const vlqBefore = sm.toVLQ().mappings;
  const vlqResult = applyReplacementsToVLQMappings(vlqBefore, replacements);
  applyReplacementsToSourceMap(sm, replacements);
  const nativeResult = sm.toVLQ().mappings;
  assert.strictEqual(
    vlqResult,
    nativeResult,
    `VLQ result differs from native:\n  VLQ:    ${vlqResult}\n  native: ${nativeResult}`,
  );
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

describe('applyReplacementsToVLQMappings', () => {
  it('returns the same string for empty replacements', () => {
    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: 10},
      original: {line: 1, column: 0},
      source: 'test.js',
    });
    const vlq = sm.toVLQ().mappings;
    assert.strictEqual(applyReplacementsToVLQMappings(vlq, []), vlq);
  });

  it('returns the same string for a zero-delta replacement', () => {
    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: 30},
      original: {line: 1, column: 0},
      source: 'test.js',
    });
    const vlq = sm.toVLQ().mappings;
    const repl: HashRefReplacement[] = [
      {line: 0, column: 0, originalLength: 10, newLength: 10},
    ];
    assert.strictEqual(applyReplacementsToVLQMappings(vlq, repl), vlq);
  });

  it('single replacement agrees with native', () => {
    const {correctReplacements, identifierPositions} = buildCodeWithHashRefs([
      {type: 'hashref'},
      {type: 'code', text: ';var x=SOME_IDENT;'},
    ]);
    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: identifierPositions.get('SOME_IDENT')!},
      original: {line: 10, column: 5},
      source: 'test.js',
    });
    crossCheck(sm, correctReplacements);
  });

  it('multiple replacements on the same line agree with native', () => {
    const {correctReplacements, identifierPositions} = buildCodeWithHashRefs([
      {type: 'hashref'},
      {type: 'code', text: ';var a=IDENT_ALPHA;require("'},
      {type: 'hashref'},
      {type: 'code', text: '");var b=IDENT_BETA;require("'},
      {type: 'hashref'},
      {type: 'code', text: '");var c=IDENT_GAMMA;'},
    ]);
    const sm = new SourceMap('/');
    for (const [name, origLine] of [
      ['IDENT_ALPHA', 10],
      ['IDENT_BETA', 20],
      ['IDENT_GAMMA', 30],
    ] as const) {
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get(name)!},
        original: {line: origLine, column: 0},
        source: 'test.js',
      });
    }
    crossCheck(sm, correctReplacements);
  });

  it('10 replacements on the same line agree with native', () => {
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

    const {correctReplacements, identifierPositions} =
      buildCodeWithHashRefs(segments);

    const sm = new SourceMap('/');
    for (let i = 0; i < 10; i++) {
      const name = `TARGET_${String(i).padStart(2, '0')}`;
      sm.addIndexedMapping({
        generated: {line: 1, column: identifierPositions.get(name)!},
        original: {line: (i + 1) * 10, column: 0},
        source: 'test.js',
      });
    }
    crossCheck(sm, correctReplacements);
  });

  it('mapping before the threshold is unaffected', () => {
    const {correctReplacements, identifierPositions} = buildCodeWithHashRefs([
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
    crossCheck(sm, correctReplacements);
  });

  it('mapping on a different line is unaffected', () => {
    const sm = new SourceMap('/');
    // Line 0 (VLQ line index 0): hash ref at col 0, mapping at col 50
    // Line 1 (VLQ line index 1): mapping at col 5 – should be untouched
    sm.addIndexedMapping({
      generated: {line: 1, column: 50},
      original: {line: 10, column: 0},
      source: 'test.js',
    });
    sm.addIndexedMapping({
      generated: {line: 2, column: 5},
      original: {line: 20, column: 0},
      source: 'test.js',
    });

    const replacements: HashRefReplacement[] = [
      {
        line: 0,
        column: 0,
        originalLength: HASH_REF_LEN,
        newLength: REPLACEMENT_LEN,
      },
    ];

    const vlqBefore = sm.toVLQ().mappings;
    const vlqResult = applyReplacementsToVLQMappings(vlqBefore, replacements);
    applyReplacementsToSourceMap(sm, replacements);

    // Verify the line-1 mapping is unchanged by checking parsed values
    const mappings = sm.getMap().mappings;
    const line2Mapping = mappings.find((m) => m.original?.line === 20);
    assert.ok(line2Mapping, 'Line 2 mapping should exist');
    assert.strictEqual(line2Mapping!.generated.column, 5);

    // Also verify VLQ agrees with native
    assert.strictEqual(vlqResult, sm.toVLQ().mappings);
  });
});

describe('SourceMapHashRefRewriteStream', () => {
  async function applyStream(
    json: string,
    replacements: HashRefReplacement[],
    chunkSize?: number,
  ): Promise<string> {
    const inputBuf = Buffer.from(json, 'utf8');
    let readable: Readable;
    if (chunkSize != null) {
      readable = new Readable({
        read() {
          let offset = 0;
          while (offset < inputBuf.length) {
            this.push(inputBuf.slice(offset, offset + chunkSize));
            offset += chunkSize;
          }
          this.push(null);
        },
      });
    } else {
      readable = Readable.from([inputBuf]);
    }
    const outBuf = await streamToBuffer(
      readable.pipe(new SourceMapHashRefRewriteStream(replacements)),
    );
    return outBuf.toString('utf8');
  }

  it('full round-trip: mappings field is correctly rewritten', async () => {
    const {correctReplacements, identifierPositions} = buildCodeWithHashRefs([
      {type: 'hashref'},
      {type: 'code', text: ';var x=SOME_IDENT;'},
    ]);

    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: identifierPositions.get('SOME_IDENT')!},
      original: {line: 10, column: 5},
      source: 'test.js',
    });

    const vlqBefore = sm.toVLQ().mappings;
    const expectedMappings = applyReplacementsToVLQMappings(
      vlqBefore,
      correctReplacements,
    );

    const mapJson = await sm.stringify({format: 'string'});
    const outputJson = await applyStream(
      mapJson as string,
      correctReplacements,
    );
    const parsed = JSON.parse(outputJson);

    assert.strictEqual(parsed.mappings, expectedMappings);
  });

  it('bytes after mappings (sourcesContent) pass through unchanged', async () => {
    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: 30},
      original: {line: 5, column: 0},
      source: 'test.js',
    });
    sm.setSourceContent('test.js', 'const x = 1;\nconst y = 2;\n');

    const replacements: HashRefReplacement[] = [
      {
        line: 0,
        column: 0,
        originalLength: HASH_REF_LEN,
        newLength: REPLACEMENT_LEN,
      },
    ];

    const mapJson = (await sm.stringify({format: 'string'})) as string;
    const outputJson = await applyStream(mapJson, replacements);
    const parsedInput = JSON.parse(mapJson);
    const parsedOutput = JSON.parse(outputJson);

    // sourcesContent must be byte-for-byte identical
    assert.deepStrictEqual(
      parsedOutput.sourcesContent,
      parsedInput.sourcesContent,
    );
    assert.deepStrictEqual(parsedOutput.sources, parsedInput.sources);
    assert.deepStrictEqual(parsedOutput.names, parsedInput.names);
  });

  it('handles chunk boundaries mid-key', async () => {
    const {correctReplacements, identifierPositions} = buildCodeWithHashRefs([
      {type: 'hashref'},
      {type: 'code', text: ';var x=SOME_IDENT;'},
    ]);

    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: identifierPositions.get('SOME_IDENT')!},
      original: {line: 10, column: 5},
      source: 'test.js',
    });

    const vlqBefore = sm.toVLQ().mappings;
    const expectedMappings = applyReplacementsToVLQMappings(
      vlqBefore,
      correctReplacements,
    );

    const mapJson = (await sm.stringify({format: 'string'})) as string;

    // Test multiple chunk sizes to exercise boundary conditions.
    for (const chunkSize of [1, 3, 7, 11, 13]) {
      const outputJson = await applyStream(
        mapJson,
        correctReplacements,
        chunkSize,
      );
      const parsed = JSON.parse(outputJson);
      assert.strictEqual(
        parsed.mappings,
        expectedMappings,
        `Chunk size ${chunkSize}: mappings mismatch`,
      );
    }
  });

  it('handles chunk boundaries mid-value', async () => {
    // Use a source map with a longer mappings string to ensure the VLQ value
    // spans multiple chunks for small chunk sizes.
    const sm = new SourceMap('/');
    for (let i = 0; i < 20; i++) {
      sm.addIndexedMapping({
        generated: {line: 1, column: i * 5},
        original: {line: i + 1, column: 0},
        source: 'test.js',
      });
    }

    const replacements: HashRefReplacement[] = [
      {
        line: 0,
        column: 10,
        originalLength: HASH_REF_LEN,
        newLength: REPLACEMENT_LEN,
      },
    ];

    const vlqBefore = sm.toVLQ().mappings;
    const expectedMappings = applyReplacementsToVLQMappings(
      vlqBefore,
      replacements,
    );

    const mapJson = (await sm.stringify({format: 'string'})) as string;

    for (const chunkSize of [1, 5, 8]) {
      const outputJson = await applyStream(mapJson, replacements, chunkSize);
      const parsed = JSON.parse(outputJson);
      assert.strictEqual(
        parsed.mappings,
        expectedMappings,
        `Chunk size ${chunkSize}: mappings mismatch`,
      );
    }
  });

  it('no-op for empty replacements – output equals input', async () => {
    const sm = new SourceMap('/');
    sm.addIndexedMapping({
      generated: {line: 1, column: 10},
      original: {line: 1, column: 0},
      source: 'test.js',
    });

    const mapJson = (await sm.stringify({format: 'string'})) as string;
    const outputJson = await applyStream(mapJson, []);

    assert.strictEqual(outputJson, mapJson);
  });
});
