// yarn pbjs --target static ./packages/core/core/src/protobuf/schema.proto --es6 --wrap es6 --out ./codegen.js

// const protobuf = require('protobufjs');

// const schema = protobuf.loadSync('packages/core/core/src/protobuf/schema.proto');

// const Dependency = schema.lookupType('Dependency');

// const dependency = Dependency.create({
//   id: '123',
//   specifier: '123',
//   specifierType: 'COMMONJS',
//   priority: 'SYNC',
//   bundleBehavior: 'INLINE',
// });

import protobuf from 'protobufjs/minimal.js';
import * as codegen from './codegen.js';

const sourceLocation = codegen.SourceLocation.fromObject({
  filePath: 'test.js',
  start: {
    line: 1,
    column: 1,
  },
  end: {
    line: 1,
    column: 1,
  },
});

const writer = protobuf.Writer.create();
console.log(sourceLocation);
codegen.SourceLocation.encode(sourceLocation, writer);

const buffer = writer.finish();
console.log(buffer);
console.log(JSON.stringify(buffer.toString()));
console.log(JSON.stringify(sourceLocation.toJSON()));
console.log(buffer.length);
console.log(JSON.stringify(sourceLocation.toJSON()).length);
