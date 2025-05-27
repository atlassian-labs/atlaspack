// Common aliases
const $Reader = $protobuf.Reader,
  $Writer = $protobuf.Writer,
  $util = $protobuf.util;

// Exported root namespace
const $root = $protobuf.roots['default'] || ($protobuf.roots['default'] = {});

/**
 * SpecifierType enum.
 * @exports SpecifierType
 * @enum {number}
 * @property {number} SPECIFIER_TYPE_COMMONJS=0 SPECIFIER_TYPE_COMMONJS value
 * @property {number} SPECIFIER_TYPE_ESM=1 SPECIFIER_TYPE_ESM value
 * @property {number} SPECIFIER_TYPE_URL=2 SPECIFIER_TYPE_URL value
 * @property {number} SPECIFIER_TYPE_CUSTOM=3 SPECIFIER_TYPE_CUSTOM value
 */
export const SpecifierType = ($root.SpecifierType = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'SPECIFIER_TYPE_COMMONJS')] = 0;
  values[(valuesById[1] = 'SPECIFIER_TYPE_ESM')] = 1;
  values[(valuesById[2] = 'SPECIFIER_TYPE_URL')] = 2;
  values[(valuesById[3] = 'SPECIFIER_TYPE_CUSTOM')] = 3;
  return values;
})());

/**
 * DependencyPriority enum.
 * @exports DependencyPriority
 * @enum {number}
 * @property {number} DEPENDENCY_PRIORITY_SYNC=0 DEPENDENCY_PRIORITY_SYNC value
 * @property {number} DEPENDENCY_PRIORITY_PARALLEL=1 DEPENDENCY_PRIORITY_PARALLEL value
 * @property {number} DEPENDENCY_PRIORITY_LAZY=2 DEPENDENCY_PRIORITY_LAZY value
 * @property {number} DEPENDENCY_PRIORITY_CONDITIONAL=3 DEPENDENCY_PRIORITY_CONDITIONAL value
 */
export const DependencyPriority = ($root.DependencyPriority = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'DEPENDENCY_PRIORITY_SYNC')] = 0;
  values[(valuesById[1] = 'DEPENDENCY_PRIORITY_PARALLEL')] = 1;
  values[(valuesById[2] = 'DEPENDENCY_PRIORITY_LAZY')] = 2;
  values[(valuesById[3] = 'DEPENDENCY_PRIORITY_CONDITIONAL')] = 3;
  return values;
})());

/**
 * BundleBehavior enum.
 * @exports BundleBehavior
 * @enum {number}
 * @property {number} BUNDLE_BEHAVIOR_INLINE=0 BUNDLE_BEHAVIOR_INLINE value
 * @property {number} BUNDLE_BEHAVIOR_ISOLATED=1 BUNDLE_BEHAVIOR_ISOLATED value
 */
export const BundleBehavior = ($root.BundleBehavior = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'BUNDLE_BEHAVIOR_INLINE')] = 0;
  values[(valuesById[1] = 'BUNDLE_BEHAVIOR_ISOLATED')] = 1;
  return values;
})());

export const ASTGenerator = ($root.ASTGenerator = (() => {
  /**
   * Properties of a ASTGenerator.
   * @exports IASTGenerator
   * @interface IASTGenerator
   * @property {string|null} [name] ASTGenerator name
   * @property {string|null} [version] ASTGenerator version
   */

  /**
   * Constructs a new ASTGenerator.
   * @exports ASTGenerator
   * @classdesc Represents a ASTGenerator.
   * @implements IASTGenerator
   * @constructor
   * @param {IASTGenerator=} [properties] Properties to set
   */
  function ASTGenerator(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * ASTGenerator name.
   * @member {string} name
   * @memberof ASTGenerator
   * @instance
   */
  ASTGenerator.prototype.name = '';

  /**
   * ASTGenerator version.
   * @member {string} version
   * @memberof ASTGenerator
   * @instance
   */
  ASTGenerator.prototype.version = '';

  /**
   * Creates a new ASTGenerator instance using the specified properties.
   * @function create
   * @memberof ASTGenerator
   * @static
   * @param {IASTGenerator=} [properties] Properties to set
   * @returns {ASTGenerator} ASTGenerator instance
   */
  ASTGenerator.create = function create(properties) {
    return new ASTGenerator(properties);
  };

  /**
   * Encodes the specified ASTGenerator message. Does not implicitly {@link ASTGenerator.verify|verify} messages.
   * @function encode
   * @memberof ASTGenerator
   * @static
   * @param {IASTGenerator} message ASTGenerator message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  ASTGenerator.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.name != null && Object.hasOwnProperty.call(message, 'name'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.name);
    if (
      message.version != null &&
      Object.hasOwnProperty.call(message, 'version')
    )
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.version);
    return writer;
  };

  /**
   * Encodes the specified ASTGenerator message, length delimited. Does not implicitly {@link ASTGenerator.verify|verify} messages.
   * @function encodeDelimited
   * @memberof ASTGenerator
   * @static
   * @param {IASTGenerator} message ASTGenerator message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  ASTGenerator.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a ASTGenerator message from the specified reader or buffer.
   * @function decode
   * @memberof ASTGenerator
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {ASTGenerator} ASTGenerator
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  ASTGenerator.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.ASTGenerator();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.name = reader.string();
          break;
        }
        case 2: {
          message.version = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a ASTGenerator message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof ASTGenerator
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {ASTGenerator} ASTGenerator
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  ASTGenerator.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a ASTGenerator message.
   * @function verify
   * @memberof ASTGenerator
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  ASTGenerator.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.name != null && message.hasOwnProperty('name'))
      if (!$util.isString(message.name)) return 'name: string expected';
    if (message.version != null && message.hasOwnProperty('version'))
      if (!$util.isString(message.version)) return 'version: string expected';
    return null;
  };

  /**
   * Creates a ASTGenerator message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof ASTGenerator
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {ASTGenerator} ASTGenerator
   */
  ASTGenerator.fromObject = function fromObject(object) {
    if (object instanceof $root.ASTGenerator) return object;
    let message = new $root.ASTGenerator();
    if (object.name != null) message.name = String(object.name);
    if (object.version != null) message.version = String(object.version);
    return message;
  };

  /**
   * Creates a plain object from a ASTGenerator message. Also converts values to other types if specified.
   * @function toObject
   * @memberof ASTGenerator
   * @static
   * @param {ASTGenerator} message ASTGenerator
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  ASTGenerator.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.name = '';
      object.version = '';
    }
    if (message.name != null && message.hasOwnProperty('name'))
      object.name = message.name;
    if (message.version != null && message.hasOwnProperty('version'))
      object.version = message.version;
    return object;
  };

  /**
   * Converts this ASTGenerator to JSON.
   * @function toJSON
   * @memberof ASTGenerator
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  ASTGenerator.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for ASTGenerator
   * @function getTypeUrl
   * @memberof ASTGenerator
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  ASTGenerator.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/ASTGenerator';
  };

  return ASTGenerator;
})());

export const LineColumn = ($root.LineColumn = (() => {
  /**
   * Properties of a LineColumn.
   * @exports ILineColumn
   * @interface ILineColumn
   * @property {number|null} [line] LineColumn line
   * @property {number|null} [column] LineColumn column
   */

  /**
   * Constructs a new LineColumn.
   * @exports LineColumn
   * @classdesc Represents a LineColumn.
   * @implements ILineColumn
   * @constructor
   * @param {ILineColumn=} [properties] Properties to set
   */
  function LineColumn(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * LineColumn line.
   * @member {number} line
   * @memberof LineColumn
   * @instance
   */
  LineColumn.prototype.line = 0;

  /**
   * LineColumn column.
   * @member {number} column
   * @memberof LineColumn
   * @instance
   */
  LineColumn.prototype.column = 0;

  /**
   * Creates a new LineColumn instance using the specified properties.
   * @function create
   * @memberof LineColumn
   * @static
   * @param {ILineColumn=} [properties] Properties to set
   * @returns {LineColumn} LineColumn instance
   */
  LineColumn.create = function create(properties) {
    return new LineColumn(properties);
  };

  /**
   * Encodes the specified LineColumn message. Does not implicitly {@link LineColumn.verify|verify} messages.
   * @function encode
   * @memberof LineColumn
   * @static
   * @param {ILineColumn} message LineColumn message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  LineColumn.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.line != null && Object.hasOwnProperty.call(message, 'line'))
      writer.uint32(/* id 1, wireType 0 =*/ 8).int32(message.line);
    if (message.column != null && Object.hasOwnProperty.call(message, 'column'))
      writer.uint32(/* id 2, wireType 0 =*/ 16).int32(message.column);
    return writer;
  };

  /**
   * Encodes the specified LineColumn message, length delimited. Does not implicitly {@link LineColumn.verify|verify} messages.
   * @function encodeDelimited
   * @memberof LineColumn
   * @static
   * @param {ILineColumn} message LineColumn message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  LineColumn.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a LineColumn message from the specified reader or buffer.
   * @function decode
   * @memberof LineColumn
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {LineColumn} LineColumn
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  LineColumn.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.LineColumn();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.line = reader.int32();
          break;
        }
        case 2: {
          message.column = reader.int32();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a LineColumn message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof LineColumn
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {LineColumn} LineColumn
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  LineColumn.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a LineColumn message.
   * @function verify
   * @memberof LineColumn
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  LineColumn.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.line != null && message.hasOwnProperty('line'))
      if (!$util.isInteger(message.line)) return 'line: integer expected';
    if (message.column != null && message.hasOwnProperty('column'))
      if (!$util.isInteger(message.column)) return 'column: integer expected';
    return null;
  };

  /**
   * Creates a LineColumn message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof LineColumn
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {LineColumn} LineColumn
   */
  LineColumn.fromObject = function fromObject(object) {
    if (object instanceof $root.LineColumn) return object;
    let message = new $root.LineColumn();
    if (object.line != null) message.line = object.line | 0;
    if (object.column != null) message.column = object.column | 0;
    return message;
  };

  /**
   * Creates a plain object from a LineColumn message. Also converts values to other types if specified.
   * @function toObject
   * @memberof LineColumn
   * @static
   * @param {LineColumn} message LineColumn
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  LineColumn.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.line = 0;
      object.column = 0;
    }
    if (message.line != null && message.hasOwnProperty('line'))
      object.line = message.line;
    if (message.column != null && message.hasOwnProperty('column'))
      object.column = message.column;
    return object;
  };

  /**
   * Converts this LineColumn to JSON.
   * @function toJSON
   * @memberof LineColumn
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  LineColumn.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for LineColumn
   * @function getTypeUrl
   * @memberof LineColumn
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  LineColumn.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/LineColumn';
  };

  return LineColumn;
})());

export const SourceLocation = ($root.SourceLocation = (() => {
  /**
   * Properties of a SourceLocation.
   * @exports ISourceLocation
   * @interface ISourceLocation
   * @property {string|null} [filePath] SourceLocation filePath
   * @property {ILineColumn|null} [start] SourceLocation start
   * @property {ILineColumn|null} [end] SourceLocation end
   */

  /**
   * Constructs a new SourceLocation.
   * @exports SourceLocation
   * @classdesc Represents a SourceLocation.
   * @implements ISourceLocation
   * @constructor
   * @param {ISourceLocation=} [properties] Properties to set
   */
  function SourceLocation(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * SourceLocation filePath.
   * @member {string} filePath
   * @memberof SourceLocation
   * @instance
   */
  SourceLocation.prototype.filePath = '';

  /**
   * SourceLocation start.
   * @member {ILineColumn|null|undefined} start
   * @memberof SourceLocation
   * @instance
   */
  SourceLocation.prototype.start = null;

  /**
   * SourceLocation end.
   * @member {ILineColumn|null|undefined} end
   * @memberof SourceLocation
   * @instance
   */
  SourceLocation.prototype.end = null;

  /**
   * Creates a new SourceLocation instance using the specified properties.
   * @function create
   * @memberof SourceLocation
   * @static
   * @param {ISourceLocation=} [properties] Properties to set
   * @returns {SourceLocation} SourceLocation instance
   */
  SourceLocation.create = function create(properties) {
    return new SourceLocation(properties);
  };

  /**
   * Encodes the specified SourceLocation message. Does not implicitly {@link SourceLocation.verify|verify} messages.
   * @function encode
   * @memberof SourceLocation
   * @static
   * @param {ISourceLocation} message SourceLocation message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  SourceLocation.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (
      message.filePath != null &&
      Object.hasOwnProperty.call(message, 'filePath')
    )
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.filePath);
    if (message.start != null && Object.hasOwnProperty.call(message, 'start'))
      $root.LineColumn.encode(
        message.start,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    if (message.end != null && Object.hasOwnProperty.call(message, 'end'))
      $root.LineColumn.encode(
        message.end,
        writer.uint32(/* id 3, wireType 2 =*/ 26).fork(),
      ).ldelim();
    return writer;
  };

  /**
   * Encodes the specified SourceLocation message, length delimited. Does not implicitly {@link SourceLocation.verify|verify} messages.
   * @function encodeDelimited
   * @memberof SourceLocation
   * @static
   * @param {ISourceLocation} message SourceLocation message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  SourceLocation.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a SourceLocation message from the specified reader or buffer.
   * @function decode
   * @memberof SourceLocation
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {SourceLocation} SourceLocation
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  SourceLocation.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.SourceLocation();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.filePath = reader.string();
          break;
        }
        case 2: {
          message.start = $root.LineColumn.decode(reader, reader.uint32());
          break;
        }
        case 3: {
          message.end = $root.LineColumn.decode(reader, reader.uint32());
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a SourceLocation message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof SourceLocation
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {SourceLocation} SourceLocation
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  SourceLocation.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a SourceLocation message.
   * @function verify
   * @memberof SourceLocation
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  SourceLocation.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      if (!$util.isString(message.filePath)) return 'filePath: string expected';
    if (message.start != null && message.hasOwnProperty('start')) {
      let error = $root.LineColumn.verify(message.start);
      if (error) return 'start.' + error;
    }
    if (message.end != null && message.hasOwnProperty('end')) {
      let error = $root.LineColumn.verify(message.end);
      if (error) return 'end.' + error;
    }
    return null;
  };

  /**
   * Creates a SourceLocation message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof SourceLocation
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {SourceLocation} SourceLocation
   */
  SourceLocation.fromObject = function fromObject(object) {
    if (object instanceof $root.SourceLocation) return object;
    let message = new $root.SourceLocation();
    if (object.filePath != null) message.filePath = String(object.filePath);
    if (object.start != null) {
      if (typeof object.start !== 'object')
        throw TypeError('.SourceLocation.start: object expected');
      message.start = $root.LineColumn.fromObject(object.start);
    }
    if (object.end != null) {
      if (typeof object.end !== 'object')
        throw TypeError('.SourceLocation.end: object expected');
      message.end = $root.LineColumn.fromObject(object.end);
    }
    return message;
  };

  /**
   * Creates a plain object from a SourceLocation message. Also converts values to other types if specified.
   * @function toObject
   * @memberof SourceLocation
   * @static
   * @param {SourceLocation} message SourceLocation
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  SourceLocation.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.filePath = '';
      object.start = null;
      object.end = null;
    }
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      object.filePath = message.filePath;
    if (message.start != null && message.hasOwnProperty('start'))
      object.start = $root.LineColumn.toObject(message.start, options);
    if (message.end != null && message.hasOwnProperty('end'))
      object.end = $root.LineColumn.toObject(message.end, options);
    return object;
  };

  /**
   * Converts this SourceLocation to JSON.
   * @function toJSON
   * @memberof SourceLocation
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  SourceLocation.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for SourceLocation
   * @function getTypeUrl
   * @memberof SourceLocation
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  SourceLocation.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/SourceLocation';
  };

  return SourceLocation;
})());

export const DependencySymbol = ($root.DependencySymbol = (() => {
  /**
   * Properties of a DependencySymbol.
   * @exports IDependencySymbol
   * @interface IDependencySymbol
   * @property {string|null} [local] DependencySymbol local
   * @property {ISourceLocation|null} [loc] DependencySymbol loc
   * @property {boolean|null} [isWeak] DependencySymbol isWeak
   * @property {string|null} [meta] DependencySymbol meta
   */

  /**
   * Constructs a new DependencySymbol.
   * @exports DependencySymbol
   * @classdesc Represents a DependencySymbol.
   * @implements IDependencySymbol
   * @constructor
   * @param {IDependencySymbol=} [properties] Properties to set
   */
  function DependencySymbol(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * DependencySymbol local.
   * @member {string} local
   * @memberof DependencySymbol
   * @instance
   */
  DependencySymbol.prototype.local = '';

  /**
   * DependencySymbol loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof DependencySymbol
   * @instance
   */
  DependencySymbol.prototype.loc = null;

  /**
   * DependencySymbol isWeak.
   * @member {boolean} isWeak
   * @memberof DependencySymbol
   * @instance
   */
  DependencySymbol.prototype.isWeak = false;

  /**
   * DependencySymbol meta.
   * @member {string} meta
   * @memberof DependencySymbol
   * @instance
   */
  DependencySymbol.prototype.meta = '';

  /**
   * Creates a new DependencySymbol instance using the specified properties.
   * @function create
   * @memberof DependencySymbol
   * @static
   * @param {IDependencySymbol=} [properties] Properties to set
   * @returns {DependencySymbol} DependencySymbol instance
   */
  DependencySymbol.create = function create(properties) {
    return new DependencySymbol(properties);
  };

  /**
   * Encodes the specified DependencySymbol message. Does not implicitly {@link DependencySymbol.verify|verify} messages.
   * @function encode
   * @memberof DependencySymbol
   * @static
   * @param {IDependencySymbol} message DependencySymbol message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  DependencySymbol.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.local != null && Object.hasOwnProperty.call(message, 'local'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.local);
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    if (message.isWeak != null && Object.hasOwnProperty.call(message, 'isWeak'))
      writer.uint32(/* id 3, wireType 0 =*/ 24).bool(message.isWeak);
    if (message.meta != null && Object.hasOwnProperty.call(message, 'meta'))
      writer.uint32(/* id 4, wireType 2 =*/ 34).string(message.meta);
    return writer;
  };

  /**
   * Encodes the specified DependencySymbol message, length delimited. Does not implicitly {@link DependencySymbol.verify|verify} messages.
   * @function encodeDelimited
   * @memberof DependencySymbol
   * @static
   * @param {IDependencySymbol} message DependencySymbol message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  DependencySymbol.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a DependencySymbol message from the specified reader or buffer.
   * @function decode
   * @memberof DependencySymbol
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {DependencySymbol} DependencySymbol
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  DependencySymbol.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.DependencySymbol();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.local = reader.string();
          break;
        }
        case 2: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        case 3: {
          message.isWeak = reader.bool();
          break;
        }
        case 4: {
          message.meta = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a DependencySymbol message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof DependencySymbol
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {DependencySymbol} DependencySymbol
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  DependencySymbol.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a DependencySymbol message.
   * @function verify
   * @memberof DependencySymbol
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  DependencySymbol.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.local != null && message.hasOwnProperty('local'))
      if (!$util.isString(message.local)) return 'local: string expected';
    if (message.loc != null && message.hasOwnProperty('loc')) {
      let error = $root.SourceLocation.verify(message.loc);
      if (error) return 'loc.' + error;
    }
    if (message.isWeak != null && message.hasOwnProperty('isWeak'))
      if (typeof message.isWeak !== 'boolean')
        return 'isWeak: boolean expected';
    if (message.meta != null && message.hasOwnProperty('meta'))
      if (!$util.isString(message.meta)) return 'meta: string expected';
    return null;
  };

  /**
   * Creates a DependencySymbol message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof DependencySymbol
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {DependencySymbol} DependencySymbol
   */
  DependencySymbol.fromObject = function fromObject(object) {
    if (object instanceof $root.DependencySymbol) return object;
    let message = new $root.DependencySymbol();
    if (object.local != null) message.local = String(object.local);
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.DependencySymbol.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    if (object.isWeak != null) message.isWeak = Boolean(object.isWeak);
    if (object.meta != null) message.meta = String(object.meta);
    return message;
  };

  /**
   * Creates a plain object from a DependencySymbol message. Also converts values to other types if specified.
   * @function toObject
   * @memberof DependencySymbol
   * @static
   * @param {DependencySymbol} message DependencySymbol
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  DependencySymbol.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.local = '';
      object.loc = null;
      object.isWeak = false;
      object.meta = '';
    }
    if (message.local != null && message.hasOwnProperty('local'))
      object.local = message.local;
    if (message.loc != null && message.hasOwnProperty('loc'))
      object.loc = $root.SourceLocation.toObject(message.loc, options);
    if (message.isWeak != null && message.hasOwnProperty('isWeak'))
      object.isWeak = message.isWeak;
    if (message.meta != null && message.hasOwnProperty('meta'))
      object.meta = message.meta;
    return object;
  };

  /**
   * Converts this DependencySymbol to JSON.
   * @function toJSON
   * @memberof DependencySymbol
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  DependencySymbol.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for DependencySymbol
   * @function getTypeUrl
   * @memberof DependencySymbol
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  DependencySymbol.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/DependencySymbol';
  };

  return DependencySymbol;
})());

export const AssetSymbol = ($root.AssetSymbol = (() => {
  /**
   * Properties of an AssetSymbol.
   * @exports IAssetSymbol
   * @interface IAssetSymbol
   * @property {string|null} [local] AssetSymbol local
   * @property {ISourceLocation|null} [loc] AssetSymbol loc
   * @property {string|null} [meta] AssetSymbol meta
   */

  /**
   * Constructs a new AssetSymbol.
   * @exports AssetSymbol
   * @classdesc Represents an AssetSymbol.
   * @implements IAssetSymbol
   * @constructor
   * @param {IAssetSymbol=} [properties] Properties to set
   */
  function AssetSymbol(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetSymbol local.
   * @member {string} local
   * @memberof AssetSymbol
   * @instance
   */
  AssetSymbol.prototype.local = '';

  /**
   * AssetSymbol loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof AssetSymbol
   * @instance
   */
  AssetSymbol.prototype.loc = null;

  /**
   * AssetSymbol meta.
   * @member {string} meta
   * @memberof AssetSymbol
   * @instance
   */
  AssetSymbol.prototype.meta = '';

  /**
   * Creates a new AssetSymbol instance using the specified properties.
   * @function create
   * @memberof AssetSymbol
   * @static
   * @param {IAssetSymbol=} [properties] Properties to set
   * @returns {AssetSymbol} AssetSymbol instance
   */
  AssetSymbol.create = function create(properties) {
    return new AssetSymbol(properties);
  };

  /**
   * Encodes the specified AssetSymbol message. Does not implicitly {@link AssetSymbol.verify|verify} messages.
   * @function encode
   * @memberof AssetSymbol
   * @static
   * @param {IAssetSymbol} message AssetSymbol message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetSymbol.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.local != null && Object.hasOwnProperty.call(message, 'local'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.local);
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    if (message.meta != null && Object.hasOwnProperty.call(message, 'meta'))
      writer.uint32(/* id 3, wireType 2 =*/ 26).string(message.meta);
    return writer;
  };

  /**
   * Encodes the specified AssetSymbol message, length delimited. Does not implicitly {@link AssetSymbol.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetSymbol
   * @static
   * @param {IAssetSymbol} message AssetSymbol message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetSymbol.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetSymbol message from the specified reader or buffer.
   * @function decode
   * @memberof AssetSymbol
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetSymbol} AssetSymbol
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetSymbol.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetSymbol();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.local = reader.string();
          break;
        }
        case 2: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        case 3: {
          message.meta = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetSymbol message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetSymbol
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetSymbol} AssetSymbol
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetSymbol.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetSymbol message.
   * @function verify
   * @memberof AssetSymbol
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetSymbol.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.local != null && message.hasOwnProperty('local'))
      if (!$util.isString(message.local)) return 'local: string expected';
    if (message.loc != null && message.hasOwnProperty('loc')) {
      let error = $root.SourceLocation.verify(message.loc);
      if (error) return 'loc.' + error;
    }
    if (message.meta != null && message.hasOwnProperty('meta'))
      if (!$util.isString(message.meta)) return 'meta: string expected';
    return null;
  };

  /**
   * Creates an AssetSymbol message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetSymbol
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetSymbol} AssetSymbol
   */
  AssetSymbol.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetSymbol) return object;
    let message = new $root.AssetSymbol();
    if (object.local != null) message.local = String(object.local);
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.AssetSymbol.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    if (object.meta != null) message.meta = String(object.meta);
    return message;
  };

  /**
   * Creates a plain object from an AssetSymbol message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetSymbol
   * @static
   * @param {AssetSymbol} message AssetSymbol
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetSymbol.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.local = '';
      object.loc = null;
      object.meta = '';
    }
    if (message.local != null && message.hasOwnProperty('local'))
      object.local = message.local;
    if (message.loc != null && message.hasOwnProperty('loc'))
      object.loc = $root.SourceLocation.toObject(message.loc, options);
    if (message.meta != null && message.hasOwnProperty('meta'))
      object.meta = message.meta;
    return object;
  };

  /**
   * Converts this AssetSymbol to JSON.
   * @function toJSON
   * @memberof AssetSymbol
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetSymbol.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetSymbol
   * @function getTypeUrl
   * @memberof AssetSymbol
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetSymbol.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetSymbol';
  };

  return AssetSymbol;
})());

export const TargetSourceMapOptions = ($root.TargetSourceMapOptions = (() => {
  /**
   * Properties of a TargetSourceMapOptions.
   * @exports ITargetSourceMapOptions
   * @interface ITargetSourceMapOptions
   * @property {string|null} [sourceRoot] TargetSourceMapOptions sourceRoot
   * @property {boolean|null} [inline] TargetSourceMapOptions inline
   * @property {boolean|null} [inlineSources] TargetSourceMapOptions inlineSources
   */

  /**
   * Constructs a new TargetSourceMapOptions.
   * @exports TargetSourceMapOptions
   * @classdesc Represents a TargetSourceMapOptions.
   * @implements ITargetSourceMapOptions
   * @constructor
   * @param {ITargetSourceMapOptions=} [properties] Properties to set
   */
  function TargetSourceMapOptions(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * TargetSourceMapOptions sourceRoot.
   * @member {string|null|undefined} sourceRoot
   * @memberof TargetSourceMapOptions
   * @instance
   */
  TargetSourceMapOptions.prototype.sourceRoot = null;

  /**
   * TargetSourceMapOptions inline.
   * @member {boolean} inline
   * @memberof TargetSourceMapOptions
   * @instance
   */
  TargetSourceMapOptions.prototype.inline = false;

  /**
   * TargetSourceMapOptions inlineSources.
   * @member {boolean} inlineSources
   * @memberof TargetSourceMapOptions
   * @instance
   */
  TargetSourceMapOptions.prototype.inlineSources = false;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(TargetSourceMapOptions.prototype, '_sourceRoot', {
    get: $util.oneOfGetter(($oneOfFields = ['sourceRoot'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new TargetSourceMapOptions instance using the specified properties.
   * @function create
   * @memberof TargetSourceMapOptions
   * @static
   * @param {ITargetSourceMapOptions=} [properties] Properties to set
   * @returns {TargetSourceMapOptions} TargetSourceMapOptions instance
   */
  TargetSourceMapOptions.create = function create(properties) {
    return new TargetSourceMapOptions(properties);
  };

  /**
   * Encodes the specified TargetSourceMapOptions message. Does not implicitly {@link TargetSourceMapOptions.verify|verify} messages.
   * @function encode
   * @memberof TargetSourceMapOptions
   * @static
   * @param {ITargetSourceMapOptions} message TargetSourceMapOptions message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  TargetSourceMapOptions.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (
      message.sourceRoot != null &&
      Object.hasOwnProperty.call(message, 'sourceRoot')
    )
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.sourceRoot);
    if (message.inline != null && Object.hasOwnProperty.call(message, 'inline'))
      writer.uint32(/* id 2, wireType 0 =*/ 16).bool(message.inline);
    if (
      message.inlineSources != null &&
      Object.hasOwnProperty.call(message, 'inlineSources')
    )
      writer.uint32(/* id 3, wireType 0 =*/ 24).bool(message.inlineSources);
    return writer;
  };

  /**
   * Encodes the specified TargetSourceMapOptions message, length delimited. Does not implicitly {@link TargetSourceMapOptions.verify|verify} messages.
   * @function encodeDelimited
   * @memberof TargetSourceMapOptions
   * @static
   * @param {ITargetSourceMapOptions} message TargetSourceMapOptions message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  TargetSourceMapOptions.encodeDelimited = function encodeDelimited(
    message,
    writer,
  ) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a TargetSourceMapOptions message from the specified reader or buffer.
   * @function decode
   * @memberof TargetSourceMapOptions
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {TargetSourceMapOptions} TargetSourceMapOptions
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  TargetSourceMapOptions.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.TargetSourceMapOptions();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.sourceRoot = reader.string();
          break;
        }
        case 2: {
          message.inline = reader.bool();
          break;
        }
        case 3: {
          message.inlineSources = reader.bool();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a TargetSourceMapOptions message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof TargetSourceMapOptions
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {TargetSourceMapOptions} TargetSourceMapOptions
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  TargetSourceMapOptions.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a TargetSourceMapOptions message.
   * @function verify
   * @memberof TargetSourceMapOptions
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  TargetSourceMapOptions.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.sourceRoot != null && message.hasOwnProperty('sourceRoot')) {
      properties._sourceRoot = 1;
      if (!$util.isString(message.sourceRoot))
        return 'sourceRoot: string expected';
    }
    if (message.inline != null && message.hasOwnProperty('inline'))
      if (typeof message.inline !== 'boolean')
        return 'inline: boolean expected';
    if (
      message.inlineSources != null &&
      message.hasOwnProperty('inlineSources')
    )
      if (typeof message.inlineSources !== 'boolean')
        return 'inlineSources: boolean expected';
    return null;
  };

  /**
   * Creates a TargetSourceMapOptions message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof TargetSourceMapOptions
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {TargetSourceMapOptions} TargetSourceMapOptions
   */
  TargetSourceMapOptions.fromObject = function fromObject(object) {
    if (object instanceof $root.TargetSourceMapOptions) return object;
    let message = new $root.TargetSourceMapOptions();
    if (object.sourceRoot != null)
      message.sourceRoot = String(object.sourceRoot);
    if (object.inline != null) message.inline = Boolean(object.inline);
    if (object.inlineSources != null)
      message.inlineSources = Boolean(object.inlineSources);
    return message;
  };

  /**
   * Creates a plain object from a TargetSourceMapOptions message. Also converts values to other types if specified.
   * @function toObject
   * @memberof TargetSourceMapOptions
   * @static
   * @param {TargetSourceMapOptions} message TargetSourceMapOptions
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  TargetSourceMapOptions.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.inline = false;
      object.inlineSources = false;
    }
    if (message.sourceRoot != null && message.hasOwnProperty('sourceRoot')) {
      object.sourceRoot = message.sourceRoot;
      if (options.oneofs) object._sourceRoot = 'sourceRoot';
    }
    if (message.inline != null && message.hasOwnProperty('inline'))
      object.inline = message.inline;
    if (
      message.inlineSources != null &&
      message.hasOwnProperty('inlineSources')
    )
      object.inlineSources = message.inlineSources;
    return object;
  };

  /**
   * Converts this TargetSourceMapOptions to JSON.
   * @function toJSON
   * @memberof TargetSourceMapOptions
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  TargetSourceMapOptions.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for TargetSourceMapOptions
   * @function getTypeUrl
   * @memberof TargetSourceMapOptions
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  TargetSourceMapOptions.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/TargetSourceMapOptions';
  };

  return TargetSourceMapOptions;
})());

export const Environment = ($root.Environment = (() => {
  /**
   * Properties of an Environment.
   * @exports IEnvironment
   * @interface IEnvironment
   * @property {string|null} [id] Environment id
   * @property {EnvironmentContext|null} [context] Environment context
   * @property {string|null} [engines] Environment engines
   * @property {string|null} [includeNodeModules] Environment includeNodeModules
   * @property {OutputFormat|null} [outputFormat] Environment outputFormat
   * @property {SourceType|null} [sourceType] Environment sourceType
   * @property {boolean|null} [isLibrary] Environment isLibrary
   * @property {boolean|null} [shouldOptimize] Environment shouldOptimize
   * @property {boolean|null} [shouldScopeHoist] Environment shouldScopeHoist
   * @property {ITargetSourceMapOptions|null} [sourceMap] Environment sourceMap
   * @property {ISourceLocation|null} [loc] Environment loc
   * @property {boolean|null} [unstableSingleFileOutput] Environment unstableSingleFileOutput
   */

  /**
   * Constructs a new Environment.
   * @exports Environment
   * @classdesc Represents an Environment.
   * @implements IEnvironment
   * @constructor
   * @param {IEnvironment=} [properties] Properties to set
   */
  function Environment(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * Environment id.
   * @member {string} id
   * @memberof Environment
   * @instance
   */
  Environment.prototype.id = '';

  /**
   * Environment context.
   * @member {EnvironmentContext} context
   * @memberof Environment
   * @instance
   */
  Environment.prototype.context = 0;

  /**
   * Environment engines.
   * @member {string} engines
   * @memberof Environment
   * @instance
   */
  Environment.prototype.engines = '';

  /**
   * Environment includeNodeModules.
   * @member {string} includeNodeModules
   * @memberof Environment
   * @instance
   */
  Environment.prototype.includeNodeModules = '';

  /**
   * Environment outputFormat.
   * @member {OutputFormat} outputFormat
   * @memberof Environment
   * @instance
   */
  Environment.prototype.outputFormat = 0;

  /**
   * Environment sourceType.
   * @member {SourceType} sourceType
   * @memberof Environment
   * @instance
   */
  Environment.prototype.sourceType = 0;

  /**
   * Environment isLibrary.
   * @member {boolean} isLibrary
   * @memberof Environment
   * @instance
   */
  Environment.prototype.isLibrary = false;

  /**
   * Environment shouldOptimize.
   * @member {boolean} shouldOptimize
   * @memberof Environment
   * @instance
   */
  Environment.prototype.shouldOptimize = false;

  /**
   * Environment shouldScopeHoist.
   * @member {boolean} shouldScopeHoist
   * @memberof Environment
   * @instance
   */
  Environment.prototype.shouldScopeHoist = false;

  /**
   * Environment sourceMap.
   * @member {ITargetSourceMapOptions|null|undefined} sourceMap
   * @memberof Environment
   * @instance
   */
  Environment.prototype.sourceMap = null;

  /**
   * Environment loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof Environment
   * @instance
   */
  Environment.prototype.loc = null;

  /**
   * Environment unstableSingleFileOutput.
   * @member {boolean} unstableSingleFileOutput
   * @memberof Environment
   * @instance
   */
  Environment.prototype.unstableSingleFileOutput = false;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Environment.prototype, '_sourceMap', {
    get: $util.oneOfGetter(($oneOfFields = ['sourceMap'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Environment.prototype, '_loc', {
    get: $util.oneOfGetter(($oneOfFields = ['loc'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new Environment instance using the specified properties.
   * @function create
   * @memberof Environment
   * @static
   * @param {IEnvironment=} [properties] Properties to set
   * @returns {Environment} Environment instance
   */
  Environment.create = function create(properties) {
    return new Environment(properties);
  };

  /**
   * Encodes the specified Environment message. Does not implicitly {@link Environment.verify|verify} messages.
   * @function encode
   * @memberof Environment
   * @static
   * @param {IEnvironment} message Environment message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Environment.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (
      message.context != null &&
      Object.hasOwnProperty.call(message, 'context')
    )
      writer.uint32(/* id 2, wireType 0 =*/ 16).int32(message.context);
    if (
      message.engines != null &&
      Object.hasOwnProperty.call(message, 'engines')
    )
      writer.uint32(/* id 3, wireType 2 =*/ 26).string(message.engines);
    if (
      message.includeNodeModules != null &&
      Object.hasOwnProperty.call(message, 'includeNodeModules')
    )
      writer
        .uint32(/* id 4, wireType 2 =*/ 34)
        .string(message.includeNodeModules);
    if (
      message.outputFormat != null &&
      Object.hasOwnProperty.call(message, 'outputFormat')
    )
      writer.uint32(/* id 5, wireType 0 =*/ 40).int32(message.outputFormat);
    if (
      message.sourceType != null &&
      Object.hasOwnProperty.call(message, 'sourceType')
    )
      writer.uint32(/* id 6, wireType 0 =*/ 48).int32(message.sourceType);
    if (
      message.isLibrary != null &&
      Object.hasOwnProperty.call(message, 'isLibrary')
    )
      writer.uint32(/* id 7, wireType 0 =*/ 56).bool(message.isLibrary);
    if (
      message.shouldOptimize != null &&
      Object.hasOwnProperty.call(message, 'shouldOptimize')
    )
      writer.uint32(/* id 8, wireType 0 =*/ 64).bool(message.shouldOptimize);
    if (
      message.shouldScopeHoist != null &&
      Object.hasOwnProperty.call(message, 'shouldScopeHoist')
    )
      writer.uint32(/* id 9, wireType 0 =*/ 72).bool(message.shouldScopeHoist);
    if (
      message.sourceMap != null &&
      Object.hasOwnProperty.call(message, 'sourceMap')
    )
      $root.TargetSourceMapOptions.encode(
        message.sourceMap,
        writer.uint32(/* id 10, wireType 2 =*/ 82).fork(),
      ).ldelim();
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 11, wireType 2 =*/ 90).fork(),
      ).ldelim();
    if (
      message.unstableSingleFileOutput != null &&
      Object.hasOwnProperty.call(message, 'unstableSingleFileOutput')
    )
      writer
        .uint32(/* id 12, wireType 0 =*/ 96)
        .bool(message.unstableSingleFileOutput);
    return writer;
  };

  /**
   * Encodes the specified Environment message, length delimited. Does not implicitly {@link Environment.verify|verify} messages.
   * @function encodeDelimited
   * @memberof Environment
   * @static
   * @param {IEnvironment} message Environment message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Environment.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an Environment message from the specified reader or buffer.
   * @function decode
   * @memberof Environment
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {Environment} Environment
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Environment.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.Environment();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.context = reader.int32();
          break;
        }
        case 3: {
          message.engines = reader.string();
          break;
        }
        case 4: {
          message.includeNodeModules = reader.string();
          break;
        }
        case 5: {
          message.outputFormat = reader.int32();
          break;
        }
        case 6: {
          message.sourceType = reader.int32();
          break;
        }
        case 7: {
          message.isLibrary = reader.bool();
          break;
        }
        case 8: {
          message.shouldOptimize = reader.bool();
          break;
        }
        case 9: {
          message.shouldScopeHoist = reader.bool();
          break;
        }
        case 10: {
          message.sourceMap = $root.TargetSourceMapOptions.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        case 11: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        case 12: {
          message.unstableSingleFileOutput = reader.bool();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an Environment message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof Environment
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {Environment} Environment
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Environment.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an Environment message.
   * @function verify
   * @memberof Environment
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  Environment.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.context != null && message.hasOwnProperty('context'))
      switch (message.context) {
        default:
          return 'context: enum value expected';
        case 0:
        case 1:
        case 2:
        case 3:
        case 4:
        case 5:
        case 6:
          break;
      }
    if (message.engines != null && message.hasOwnProperty('engines'))
      if (!$util.isString(message.engines)) return 'engines: string expected';
    if (
      message.includeNodeModules != null &&
      message.hasOwnProperty('includeNodeModules')
    )
      if (!$util.isString(message.includeNodeModules))
        return 'includeNodeModules: string expected';
    if (message.outputFormat != null && message.hasOwnProperty('outputFormat'))
      switch (message.outputFormat) {
        default:
          return 'outputFormat: enum value expected';
        case 0:
        case 1:
        case 2:
          break;
      }
    if (message.sourceType != null && message.hasOwnProperty('sourceType'))
      switch (message.sourceType) {
        default:
          return 'sourceType: enum value expected';
        case 0:
        case 1:
          break;
      }
    if (message.isLibrary != null && message.hasOwnProperty('isLibrary'))
      if (typeof message.isLibrary !== 'boolean')
        return 'isLibrary: boolean expected';
    if (
      message.shouldOptimize != null &&
      message.hasOwnProperty('shouldOptimize')
    )
      if (typeof message.shouldOptimize !== 'boolean')
        return 'shouldOptimize: boolean expected';
    if (
      message.shouldScopeHoist != null &&
      message.hasOwnProperty('shouldScopeHoist')
    )
      if (typeof message.shouldScopeHoist !== 'boolean')
        return 'shouldScopeHoist: boolean expected';
    if (message.sourceMap != null && message.hasOwnProperty('sourceMap')) {
      properties._sourceMap = 1;
      {
        let error = $root.TargetSourceMapOptions.verify(message.sourceMap);
        if (error) return 'sourceMap.' + error;
      }
    }
    if (message.loc != null && message.hasOwnProperty('loc')) {
      properties._loc = 1;
      {
        let error = $root.SourceLocation.verify(message.loc);
        if (error) return 'loc.' + error;
      }
    }
    if (
      message.unstableSingleFileOutput != null &&
      message.hasOwnProperty('unstableSingleFileOutput')
    )
      if (typeof message.unstableSingleFileOutput !== 'boolean')
        return 'unstableSingleFileOutput: boolean expected';
    return null;
  };

  /**
   * Creates an Environment message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof Environment
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {Environment} Environment
   */
  Environment.fromObject = function fromObject(object) {
    if (object instanceof $root.Environment) return object;
    let message = new $root.Environment();
    if (object.id != null) message.id = String(object.id);
    switch (object.context) {
      default:
        if (typeof object.context === 'number') {
          message.context = object.context;
          break;
        }
        break;
      case 'ENVIRONMENT_CONTEXT_BROWSER':
      case 0:
        message.context = 0;
        break;
      case 'ENVIRONMENT_CONTEXT_WEB_WORKER':
      case 1:
        message.context = 1;
        break;
      case 'ENVIRONMENT_CONTEXT_SERVICE_WORKER':
      case 2:
        message.context = 2;
        break;
      case 'ENVIRONMENT_CONTEXT_WORKLET':
      case 3:
        message.context = 3;
        break;
      case 'ENVIRONMENT_CONTEXT_NODE':
      case 4:
        message.context = 4;
        break;
      case 'ENVIRONMENT_CONTEXT_ELECTRON_MAIN':
      case 5:
        message.context = 5;
        break;
      case 'ENVIRONMENT_CONTEXT_ELECTRON_RENDERER':
      case 6:
        message.context = 6;
        break;
    }
    if (object.engines != null) message.engines = String(object.engines);
    if (object.includeNodeModules != null)
      message.includeNodeModules = String(object.includeNodeModules);
    switch (object.outputFormat) {
      default:
        if (typeof object.outputFormat === 'number') {
          message.outputFormat = object.outputFormat;
          break;
        }
        break;
      case 'OUTPUT_FORMAT_ESMODULE':
      case 0:
        message.outputFormat = 0;
        break;
      case 'OUTPUT_FORMAT_COMMONJS':
      case 1:
        message.outputFormat = 1;
        break;
      case 'OUTPUT_FORMAT_GLOBAL':
      case 2:
        message.outputFormat = 2;
        break;
    }
    switch (object.sourceType) {
      default:
        if (typeof object.sourceType === 'number') {
          message.sourceType = object.sourceType;
          break;
        }
        break;
      case 'SOURCE_TYPE_SCRIPT':
      case 0:
        message.sourceType = 0;
        break;
      case 'SOURCE_TYPE_MODULE':
      case 1:
        message.sourceType = 1;
        break;
    }
    if (object.isLibrary != null) message.isLibrary = Boolean(object.isLibrary);
    if (object.shouldOptimize != null)
      message.shouldOptimize = Boolean(object.shouldOptimize);
    if (object.shouldScopeHoist != null)
      message.shouldScopeHoist = Boolean(object.shouldScopeHoist);
    if (object.sourceMap != null) {
      if (typeof object.sourceMap !== 'object')
        throw TypeError('.Environment.sourceMap: object expected');
      message.sourceMap = $root.TargetSourceMapOptions.fromObject(
        object.sourceMap,
      );
    }
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.Environment.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    if (object.unstableSingleFileOutput != null)
      message.unstableSingleFileOutput = Boolean(
        object.unstableSingleFileOutput,
      );
    return message;
  };

  /**
   * Creates a plain object from an Environment message. Also converts values to other types if specified.
   * @function toObject
   * @memberof Environment
   * @static
   * @param {Environment} message Environment
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  Environment.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.id = '';
      object.context =
        options.enums === String ? 'ENVIRONMENT_CONTEXT_BROWSER' : 0;
      object.engines = '';
      object.includeNodeModules = '';
      object.outputFormat =
        options.enums === String ? 'OUTPUT_FORMAT_ESMODULE' : 0;
      object.sourceType = options.enums === String ? 'SOURCE_TYPE_SCRIPT' : 0;
      object.isLibrary = false;
      object.shouldOptimize = false;
      object.shouldScopeHoist = false;
      object.unstableSingleFileOutput = false;
    }
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.context != null && message.hasOwnProperty('context'))
      object.context =
        options.enums === String
          ? $root.EnvironmentContext[message.context] === undefined
            ? message.context
            : $root.EnvironmentContext[message.context]
          : message.context;
    if (message.engines != null && message.hasOwnProperty('engines'))
      object.engines = message.engines;
    if (
      message.includeNodeModules != null &&
      message.hasOwnProperty('includeNodeModules')
    )
      object.includeNodeModules = message.includeNodeModules;
    if (message.outputFormat != null && message.hasOwnProperty('outputFormat'))
      object.outputFormat =
        options.enums === String
          ? $root.OutputFormat[message.outputFormat] === undefined
            ? message.outputFormat
            : $root.OutputFormat[message.outputFormat]
          : message.outputFormat;
    if (message.sourceType != null && message.hasOwnProperty('sourceType'))
      object.sourceType =
        options.enums === String
          ? $root.SourceType[message.sourceType] === undefined
            ? message.sourceType
            : $root.SourceType[message.sourceType]
          : message.sourceType;
    if (message.isLibrary != null && message.hasOwnProperty('isLibrary'))
      object.isLibrary = message.isLibrary;
    if (
      message.shouldOptimize != null &&
      message.hasOwnProperty('shouldOptimize')
    )
      object.shouldOptimize = message.shouldOptimize;
    if (
      message.shouldScopeHoist != null &&
      message.hasOwnProperty('shouldScopeHoist')
    )
      object.shouldScopeHoist = message.shouldScopeHoist;
    if (message.sourceMap != null && message.hasOwnProperty('sourceMap')) {
      object.sourceMap = $root.TargetSourceMapOptions.toObject(
        message.sourceMap,
        options,
      );
      if (options.oneofs) object._sourceMap = 'sourceMap';
    }
    if (message.loc != null && message.hasOwnProperty('loc')) {
      object.loc = $root.SourceLocation.toObject(message.loc, options);
      if (options.oneofs) object._loc = 'loc';
    }
    if (
      message.unstableSingleFileOutput != null &&
      message.hasOwnProperty('unstableSingleFileOutput')
    )
      object.unstableSingleFileOutput = message.unstableSingleFileOutput;
    return object;
  };

  /**
   * Converts this Environment to JSON.
   * @function toJSON
   * @memberof Environment
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  Environment.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for Environment
   * @function getTypeUrl
   * @memberof Environment
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  Environment.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/Environment';
  };

  return Environment;
})());

export const Target = ($root.Target = (() => {
  /**
   * Properties of a Target.
   * @exports ITarget
   * @interface ITarget
   * @property {string|null} [distEntry] Target distEntry
   * @property {string|null} [distDir] Target distDir
   * @property {IEnvironment|null} [env] Target env
   * @property {string|null} [name] Target name
   * @property {string|null} [publicUrl] Target publicUrl
   * @property {ISourceLocation|null} [loc] Target loc
   */

  /**
   * Constructs a new Target.
   * @exports Target
   * @classdesc Represents a Target.
   * @implements ITarget
   * @constructor
   * @param {ITarget=} [properties] Properties to set
   */
  function Target(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * Target distEntry.
   * @member {string|null|undefined} distEntry
   * @memberof Target
   * @instance
   */
  Target.prototype.distEntry = null;

  /**
   * Target distDir.
   * @member {string} distDir
   * @memberof Target
   * @instance
   */
  Target.prototype.distDir = '';

  /**
   * Target env.
   * @member {IEnvironment|null|undefined} env
   * @memberof Target
   * @instance
   */
  Target.prototype.env = null;

  /**
   * Target name.
   * @member {string} name
   * @memberof Target
   * @instance
   */
  Target.prototype.name = '';

  /**
   * Target publicUrl.
   * @member {string} publicUrl
   * @memberof Target
   * @instance
   */
  Target.prototype.publicUrl = '';

  /**
   * Target loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof Target
   * @instance
   */
  Target.prototype.loc = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Target.prototype, '_distEntry', {
    get: $util.oneOfGetter(($oneOfFields = ['distEntry'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Target.prototype, '_loc', {
    get: $util.oneOfGetter(($oneOfFields = ['loc'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new Target instance using the specified properties.
   * @function create
   * @memberof Target
   * @static
   * @param {ITarget=} [properties] Properties to set
   * @returns {Target} Target instance
   */
  Target.create = function create(properties) {
    return new Target(properties);
  };

  /**
   * Encodes the specified Target message. Does not implicitly {@link Target.verify|verify} messages.
   * @function encode
   * @memberof Target
   * @static
   * @param {ITarget} message Target message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Target.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (
      message.distEntry != null &&
      Object.hasOwnProperty.call(message, 'distEntry')
    )
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.distEntry);
    if (
      message.distDir != null &&
      Object.hasOwnProperty.call(message, 'distDir')
    )
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.distDir);
    if (message.env != null && Object.hasOwnProperty.call(message, 'env'))
      $root.Environment.encode(
        message.env,
        writer.uint32(/* id 3, wireType 2 =*/ 26).fork(),
      ).ldelim();
    if (message.name != null && Object.hasOwnProperty.call(message, 'name'))
      writer.uint32(/* id 4, wireType 2 =*/ 34).string(message.name);
    if (
      message.publicUrl != null &&
      Object.hasOwnProperty.call(message, 'publicUrl')
    )
      writer.uint32(/* id 5, wireType 2 =*/ 42).string(message.publicUrl);
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 6, wireType 2 =*/ 50).fork(),
      ).ldelim();
    return writer;
  };

  /**
   * Encodes the specified Target message, length delimited. Does not implicitly {@link Target.verify|verify} messages.
   * @function encodeDelimited
   * @memberof Target
   * @static
   * @param {ITarget} message Target message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Target.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a Target message from the specified reader or buffer.
   * @function decode
   * @memberof Target
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {Target} Target
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Target.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.Target();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.distEntry = reader.string();
          break;
        }
        case 2: {
          message.distDir = reader.string();
          break;
        }
        case 3: {
          message.env = $root.Environment.decode(reader, reader.uint32());
          break;
        }
        case 4: {
          message.name = reader.string();
          break;
        }
        case 5: {
          message.publicUrl = reader.string();
          break;
        }
        case 6: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a Target message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof Target
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {Target} Target
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Target.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a Target message.
   * @function verify
   * @memberof Target
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  Target.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.distEntry != null && message.hasOwnProperty('distEntry')) {
      properties._distEntry = 1;
      if (!$util.isString(message.distEntry))
        return 'distEntry: string expected';
    }
    if (message.distDir != null && message.hasOwnProperty('distDir'))
      if (!$util.isString(message.distDir)) return 'distDir: string expected';
    if (message.env != null && message.hasOwnProperty('env')) {
      let error = $root.Environment.verify(message.env);
      if (error) return 'env.' + error;
    }
    if (message.name != null && message.hasOwnProperty('name'))
      if (!$util.isString(message.name)) return 'name: string expected';
    if (message.publicUrl != null && message.hasOwnProperty('publicUrl'))
      if (!$util.isString(message.publicUrl))
        return 'publicUrl: string expected';
    if (message.loc != null && message.hasOwnProperty('loc')) {
      properties._loc = 1;
      {
        let error = $root.SourceLocation.verify(message.loc);
        if (error) return 'loc.' + error;
      }
    }
    return null;
  };

  /**
   * Creates a Target message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof Target
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {Target} Target
   */
  Target.fromObject = function fromObject(object) {
    if (object instanceof $root.Target) return object;
    let message = new $root.Target();
    if (object.distEntry != null) message.distEntry = String(object.distEntry);
    if (object.distDir != null) message.distDir = String(object.distDir);
    if (object.env != null) {
      if (typeof object.env !== 'object')
        throw TypeError('.Target.env: object expected');
      message.env = $root.Environment.fromObject(object.env);
    }
    if (object.name != null) message.name = String(object.name);
    if (object.publicUrl != null) message.publicUrl = String(object.publicUrl);
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.Target.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    return message;
  };

  /**
   * Creates a plain object from a Target message. Also converts values to other types if specified.
   * @function toObject
   * @memberof Target
   * @static
   * @param {Target} message Target
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  Target.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.distDir = '';
      object.env = null;
      object.name = '';
      object.publicUrl = '';
    }
    if (message.distEntry != null && message.hasOwnProperty('distEntry')) {
      object.distEntry = message.distEntry;
      if (options.oneofs) object._distEntry = 'distEntry';
    }
    if (message.distDir != null && message.hasOwnProperty('distDir'))
      object.distDir = message.distDir;
    if (message.env != null && message.hasOwnProperty('env'))
      object.env = $root.Environment.toObject(message.env, options);
    if (message.name != null && message.hasOwnProperty('name'))
      object.name = message.name;
    if (message.publicUrl != null && message.hasOwnProperty('publicUrl'))
      object.publicUrl = message.publicUrl;
    if (message.loc != null && message.hasOwnProperty('loc')) {
      object.loc = $root.SourceLocation.toObject(message.loc, options);
      if (options.oneofs) object._loc = 'loc';
    }
    return object;
  };

  /**
   * Converts this Target to JSON.
   * @function toJSON
   * @memberof Target
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  Target.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for Target
   * @function getTypeUrl
   * @memberof Target
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  Target.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/Target';
  };

  return Target;
})());

export const AssetGraphEntrySpecifierNode =
  ($root.AssetGraphEntrySpecifierNode = (() => {
    /**
     * Properties of an AssetGraphEntrySpecifierNode.
     * @exports IAssetGraphEntrySpecifierNode
     * @interface IAssetGraphEntrySpecifierNode
     * @property {string|null} [id] AssetGraphEntrySpecifierNode id
     * @property {string|null} [value] AssetGraphEntrySpecifierNode value
     * @property {string|null} [correspondingRequest] AssetGraphEntrySpecifierNode correspondingRequest
     */

    /**
     * Constructs a new AssetGraphEntrySpecifierNode.
     * @exports AssetGraphEntrySpecifierNode
     * @classdesc Represents an AssetGraphEntrySpecifierNode.
     * @implements IAssetGraphEntrySpecifierNode
     * @constructor
     * @param {IAssetGraphEntrySpecifierNode=} [properties] Properties to set
     */
    function AssetGraphEntrySpecifierNode(properties) {
      if (properties)
        for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
          if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
    }

    /**
     * AssetGraphEntrySpecifierNode id.
     * @member {string} id
     * @memberof AssetGraphEntrySpecifierNode
     * @instance
     */
    AssetGraphEntrySpecifierNode.prototype.id = '';

    /**
     * AssetGraphEntrySpecifierNode value.
     * @member {string} value
     * @memberof AssetGraphEntrySpecifierNode
     * @instance
     */
    AssetGraphEntrySpecifierNode.prototype.value = '';

    /**
     * AssetGraphEntrySpecifierNode correspondingRequest.
     * @member {string|null|undefined} correspondingRequest
     * @memberof AssetGraphEntrySpecifierNode
     * @instance
     */
    AssetGraphEntrySpecifierNode.prototype.correspondingRequest = null;

    // OneOf field names bound to virtual getters and setters
    let $oneOfFields;

    // Virtual OneOf for proto3 optional field
    Object.defineProperty(
      AssetGraphEntrySpecifierNode.prototype,
      '_correspondingRequest',
      {
        get: $util.oneOfGetter(($oneOfFields = ['correspondingRequest'])),
        set: $util.oneOfSetter($oneOfFields),
      },
    );

    /**
     * Creates a new AssetGraphEntrySpecifierNode instance using the specified properties.
     * @function create
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {IAssetGraphEntrySpecifierNode=} [properties] Properties to set
     * @returns {AssetGraphEntrySpecifierNode} AssetGraphEntrySpecifierNode instance
     */
    AssetGraphEntrySpecifierNode.create = function create(properties) {
      return new AssetGraphEntrySpecifierNode(properties);
    };

    /**
     * Encodes the specified AssetGraphEntrySpecifierNode message. Does not implicitly {@link AssetGraphEntrySpecifierNode.verify|verify} messages.
     * @function encode
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {IAssetGraphEntrySpecifierNode} message AssetGraphEntrySpecifierNode message or plain object to encode
     * @param {$protobuf.Writer} [writer] Writer to encode to
     * @returns {$protobuf.Writer} Writer
     */
    AssetGraphEntrySpecifierNode.encode = function encode(message, writer) {
      if (!writer) writer = $Writer.create();
      if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
        writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
      if (message.value != null && Object.hasOwnProperty.call(message, 'value'))
        writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.value);
      if (
        message.correspondingRequest != null &&
        Object.hasOwnProperty.call(message, 'correspondingRequest')
      )
        writer
          .uint32(/* id 3, wireType 2 =*/ 26)
          .string(message.correspondingRequest);
      return writer;
    };

    /**
     * Encodes the specified AssetGraphEntrySpecifierNode message, length delimited. Does not implicitly {@link AssetGraphEntrySpecifierNode.verify|verify} messages.
     * @function encodeDelimited
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {IAssetGraphEntrySpecifierNode} message AssetGraphEntrySpecifierNode message or plain object to encode
     * @param {$protobuf.Writer} [writer] Writer to encode to
     * @returns {$protobuf.Writer} Writer
     */
    AssetGraphEntrySpecifierNode.encodeDelimited = function encodeDelimited(
      message,
      writer,
    ) {
      return this.encode(message, writer).ldelim();
    };

    /**
     * Decodes an AssetGraphEntrySpecifierNode message from the specified reader or buffer.
     * @function decode
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
     * @param {number} [length] Message length if known beforehand
     * @returns {AssetGraphEntrySpecifierNode} AssetGraphEntrySpecifierNode
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    AssetGraphEntrySpecifierNode.decode = function decode(reader, length) {
      if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
      let end = length === undefined ? reader.len : reader.pos + length,
        message = new $root.AssetGraphEntrySpecifierNode();
      while (reader.pos < end) {
        let tag = reader.uint32();
        switch (tag >>> 3) {
          case 1: {
            message.id = reader.string();
            break;
          }
          case 2: {
            message.value = reader.string();
            break;
          }
          case 3: {
            message.correspondingRequest = reader.string();
            break;
          }
          default:
            reader.skipType(tag & 7);
            break;
        }
      }
      return message;
    };

    /**
     * Decodes an AssetGraphEntrySpecifierNode message from the specified reader or buffer, length delimited.
     * @function decodeDelimited
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
     * @returns {AssetGraphEntrySpecifierNode} AssetGraphEntrySpecifierNode
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    AssetGraphEntrySpecifierNode.decodeDelimited = function decodeDelimited(
      reader,
    ) {
      if (!(reader instanceof $Reader)) reader = new $Reader(reader);
      return this.decode(reader, reader.uint32());
    };

    /**
     * Verifies an AssetGraphEntrySpecifierNode message.
     * @function verify
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {Object.<string,*>} message Plain object to verify
     * @returns {string|null} `null` if valid, otherwise the reason why it is not
     */
    AssetGraphEntrySpecifierNode.verify = function verify(message) {
      if (typeof message !== 'object' || message === null)
        return 'object expected';
      let properties = {};
      if (message.id != null && message.hasOwnProperty('id'))
        if (!$util.isString(message.id)) return 'id: string expected';
      if (message.value != null && message.hasOwnProperty('value'))
        if (!$util.isString(message.value)) return 'value: string expected';
      if (
        message.correspondingRequest != null &&
        message.hasOwnProperty('correspondingRequest')
      ) {
        properties._correspondingRequest = 1;
        if (!$util.isString(message.correspondingRequest))
          return 'correspondingRequest: string expected';
      }
      return null;
    };

    /**
     * Creates an AssetGraphEntrySpecifierNode message from a plain object. Also converts values to their respective internal types.
     * @function fromObject
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {Object.<string,*>} object Plain object
     * @returns {AssetGraphEntrySpecifierNode} AssetGraphEntrySpecifierNode
     */
    AssetGraphEntrySpecifierNode.fromObject = function fromObject(object) {
      if (object instanceof $root.AssetGraphEntrySpecifierNode) return object;
      let message = new $root.AssetGraphEntrySpecifierNode();
      if (object.id != null) message.id = String(object.id);
      if (object.value != null) message.value = String(object.value);
      if (object.correspondingRequest != null)
        message.correspondingRequest = String(object.correspondingRequest);
      return message;
    };

    /**
     * Creates a plain object from an AssetGraphEntrySpecifierNode message. Also converts values to other types if specified.
     * @function toObject
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {AssetGraphEntrySpecifierNode} message AssetGraphEntrySpecifierNode
     * @param {$protobuf.IConversionOptions} [options] Conversion options
     * @returns {Object.<string,*>} Plain object
     */
    AssetGraphEntrySpecifierNode.toObject = function toObject(
      message,
      options,
    ) {
      if (!options) options = {};
      let object = {};
      if (options.defaults) {
        object.id = '';
        object.value = '';
      }
      if (message.id != null && message.hasOwnProperty('id'))
        object.id = message.id;
      if (message.value != null && message.hasOwnProperty('value'))
        object.value = message.value;
      if (
        message.correspondingRequest != null &&
        message.hasOwnProperty('correspondingRequest')
      ) {
        object.correspondingRequest = message.correspondingRequest;
        if (options.oneofs)
          object._correspondingRequest = 'correspondingRequest';
      }
      return object;
    };

    /**
     * Converts this AssetGraphEntrySpecifierNode to JSON.
     * @function toJSON
     * @memberof AssetGraphEntrySpecifierNode
     * @instance
     * @returns {Object.<string,*>} JSON object
     */
    AssetGraphEntrySpecifierNode.prototype.toJSON = function toJSON() {
      return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
    };

    /**
     * Gets the default type url for AssetGraphEntrySpecifierNode
     * @function getTypeUrl
     * @memberof AssetGraphEntrySpecifierNode
     * @static
     * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
     * @returns {string} The default type url
     */
    AssetGraphEntrySpecifierNode.getTypeUrl = function getTypeUrl(
      typeUrlPrefix,
    ) {
      if (typeUrlPrefix === undefined) {
        typeUrlPrefix = 'type.googleapis.com';
      }
      return typeUrlPrefix + '/AssetGraphEntrySpecifierNode';
    };

    return AssetGraphEntrySpecifierNode;
  })());

export const Entry = ($root.Entry = (() => {
  /**
   * Properties of an Entry.
   * @exports IEntry
   * @interface IEntry
   * @property {string|null} [filePath] Entry filePath
   * @property {string|null} [packagePath] Entry packagePath
   * @property {string|null} [target] Entry target
   * @property {ISourceLocation|null} [loc] Entry loc
   */

  /**
   * Constructs a new Entry.
   * @exports Entry
   * @classdesc Represents an Entry.
   * @implements IEntry
   * @constructor
   * @param {IEntry=} [properties] Properties to set
   */
  function Entry(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * Entry filePath.
   * @member {string} filePath
   * @memberof Entry
   * @instance
   */
  Entry.prototype.filePath = '';

  /**
   * Entry packagePath.
   * @member {string} packagePath
   * @memberof Entry
   * @instance
   */
  Entry.prototype.packagePath = '';

  /**
   * Entry target.
   * @member {string|null|undefined} target
   * @memberof Entry
   * @instance
   */
  Entry.prototype.target = null;

  /**
   * Entry loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof Entry
   * @instance
   */
  Entry.prototype.loc = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Entry.prototype, '_target', {
    get: $util.oneOfGetter(($oneOfFields = ['target'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Entry.prototype, '_loc', {
    get: $util.oneOfGetter(($oneOfFields = ['loc'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new Entry instance using the specified properties.
   * @function create
   * @memberof Entry
   * @static
   * @param {IEntry=} [properties] Properties to set
   * @returns {Entry} Entry instance
   */
  Entry.create = function create(properties) {
    return new Entry(properties);
  };

  /**
   * Encodes the specified Entry message. Does not implicitly {@link Entry.verify|verify} messages.
   * @function encode
   * @memberof Entry
   * @static
   * @param {IEntry} message Entry message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Entry.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (
      message.filePath != null &&
      Object.hasOwnProperty.call(message, 'filePath')
    )
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.filePath);
    if (
      message.packagePath != null &&
      Object.hasOwnProperty.call(message, 'packagePath')
    )
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.packagePath);
    if (message.target != null && Object.hasOwnProperty.call(message, 'target'))
      writer.uint32(/* id 3, wireType 2 =*/ 26).string(message.target);
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 4, wireType 2 =*/ 34).fork(),
      ).ldelim();
    return writer;
  };

  /**
   * Encodes the specified Entry message, length delimited. Does not implicitly {@link Entry.verify|verify} messages.
   * @function encodeDelimited
   * @memberof Entry
   * @static
   * @param {IEntry} message Entry message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Entry.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an Entry message from the specified reader or buffer.
   * @function decode
   * @memberof Entry
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {Entry} Entry
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Entry.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.Entry();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.filePath = reader.string();
          break;
        }
        case 2: {
          message.packagePath = reader.string();
          break;
        }
        case 3: {
          message.target = reader.string();
          break;
        }
        case 4: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an Entry message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof Entry
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {Entry} Entry
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Entry.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an Entry message.
   * @function verify
   * @memberof Entry
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  Entry.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      if (!$util.isString(message.filePath)) return 'filePath: string expected';
    if (message.packagePath != null && message.hasOwnProperty('packagePath'))
      if (!$util.isString(message.packagePath))
        return 'packagePath: string expected';
    if (message.target != null && message.hasOwnProperty('target')) {
      properties._target = 1;
      if (!$util.isString(message.target)) return 'target: string expected';
    }
    if (message.loc != null && message.hasOwnProperty('loc')) {
      properties._loc = 1;
      {
        let error = $root.SourceLocation.verify(message.loc);
        if (error) return 'loc.' + error;
      }
    }
    return null;
  };

  /**
   * Creates an Entry message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof Entry
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {Entry} Entry
   */
  Entry.fromObject = function fromObject(object) {
    if (object instanceof $root.Entry) return object;
    let message = new $root.Entry();
    if (object.filePath != null) message.filePath = String(object.filePath);
    if (object.packagePath != null)
      message.packagePath = String(object.packagePath);
    if (object.target != null) message.target = String(object.target);
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.Entry.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    return message;
  };

  /**
   * Creates a plain object from an Entry message. Also converts values to other types if specified.
   * @function toObject
   * @memberof Entry
   * @static
   * @param {Entry} message Entry
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  Entry.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.filePath = '';
      object.packagePath = '';
    }
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      object.filePath = message.filePath;
    if (message.packagePath != null && message.hasOwnProperty('packagePath'))
      object.packagePath = message.packagePath;
    if (message.target != null && message.hasOwnProperty('target')) {
      object.target = message.target;
      if (options.oneofs) object._target = 'target';
    }
    if (message.loc != null && message.hasOwnProperty('loc')) {
      object.loc = $root.SourceLocation.toObject(message.loc, options);
      if (options.oneofs) object._loc = 'loc';
    }
    return object;
  };

  /**
   * Converts this Entry to JSON.
   * @function toJSON
   * @memberof Entry
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  Entry.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for Entry
   * @function getTypeUrl
   * @memberof Entry
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  Entry.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/Entry';
  };

  return Entry;
})());

export const AssetGraphEntryFileNode = ($root.AssetGraphEntryFileNode = (() => {
  /**
   * Properties of an AssetGraphEntryFileNode.
   * @exports IAssetGraphEntryFileNode
   * @interface IAssetGraphEntryFileNode
   * @property {string|null} [id] AssetGraphEntryFileNode id
   * @property {IEntry|null} [value] AssetGraphEntryFileNode value
   * @property {string|null} [correspondingRequest] AssetGraphEntryFileNode correspondingRequest
   */

  /**
   * Constructs a new AssetGraphEntryFileNode.
   * @exports AssetGraphEntryFileNode
   * @classdesc Represents an AssetGraphEntryFileNode.
   * @implements IAssetGraphEntryFileNode
   * @constructor
   * @param {IAssetGraphEntryFileNode=} [properties] Properties to set
   */
  function AssetGraphEntryFileNode(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetGraphEntryFileNode id.
   * @member {string} id
   * @memberof AssetGraphEntryFileNode
   * @instance
   */
  AssetGraphEntryFileNode.prototype.id = '';

  /**
   * AssetGraphEntryFileNode value.
   * @member {IEntry|null|undefined} value
   * @memberof AssetGraphEntryFileNode
   * @instance
   */
  AssetGraphEntryFileNode.prototype.value = null;

  /**
   * AssetGraphEntryFileNode correspondingRequest.
   * @member {string|null|undefined} correspondingRequest
   * @memberof AssetGraphEntryFileNode
   * @instance
   */
  AssetGraphEntryFileNode.prototype.correspondingRequest = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(
    AssetGraphEntryFileNode.prototype,
    '_correspondingRequest',
    {
      get: $util.oneOfGetter(($oneOfFields = ['correspondingRequest'])),
      set: $util.oneOfSetter($oneOfFields),
    },
  );

  /**
   * Creates a new AssetGraphEntryFileNode instance using the specified properties.
   * @function create
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {IAssetGraphEntryFileNode=} [properties] Properties to set
   * @returns {AssetGraphEntryFileNode} AssetGraphEntryFileNode instance
   */
  AssetGraphEntryFileNode.create = function create(properties) {
    return new AssetGraphEntryFileNode(properties);
  };

  /**
   * Encodes the specified AssetGraphEntryFileNode message. Does not implicitly {@link AssetGraphEntryFileNode.verify|verify} messages.
   * @function encode
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {IAssetGraphEntryFileNode} message AssetGraphEntryFileNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphEntryFileNode.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (message.value != null && Object.hasOwnProperty.call(message, 'value'))
      $root.Entry.encode(
        message.value,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    if (
      message.correspondingRequest != null &&
      Object.hasOwnProperty.call(message, 'correspondingRequest')
    )
      writer
        .uint32(/* id 3, wireType 2 =*/ 26)
        .string(message.correspondingRequest);
    return writer;
  };

  /**
   * Encodes the specified AssetGraphEntryFileNode message, length delimited. Does not implicitly {@link AssetGraphEntryFileNode.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {IAssetGraphEntryFileNode} message AssetGraphEntryFileNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphEntryFileNode.encodeDelimited = function encodeDelimited(
    message,
    writer,
  ) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetGraphEntryFileNode message from the specified reader or buffer.
   * @function decode
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetGraphEntryFileNode} AssetGraphEntryFileNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphEntryFileNode.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetGraphEntryFileNode();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.value = $root.Entry.decode(reader, reader.uint32());
          break;
        }
        case 3: {
          message.correspondingRequest = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetGraphEntryFileNode message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetGraphEntryFileNode} AssetGraphEntryFileNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphEntryFileNode.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetGraphEntryFileNode message.
   * @function verify
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetGraphEntryFileNode.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.value != null && message.hasOwnProperty('value')) {
      let error = $root.Entry.verify(message.value);
      if (error) return 'value.' + error;
    }
    if (
      message.correspondingRequest != null &&
      message.hasOwnProperty('correspondingRequest')
    ) {
      properties._correspondingRequest = 1;
      if (!$util.isString(message.correspondingRequest))
        return 'correspondingRequest: string expected';
    }
    return null;
  };

  /**
   * Creates an AssetGraphEntryFileNode message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetGraphEntryFileNode} AssetGraphEntryFileNode
   */
  AssetGraphEntryFileNode.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetGraphEntryFileNode) return object;
    let message = new $root.AssetGraphEntryFileNode();
    if (object.id != null) message.id = String(object.id);
    if (object.value != null) {
      if (typeof object.value !== 'object')
        throw TypeError('.AssetGraphEntryFileNode.value: object expected');
      message.value = $root.Entry.fromObject(object.value);
    }
    if (object.correspondingRequest != null)
      message.correspondingRequest = String(object.correspondingRequest);
    return message;
  };

  /**
   * Creates a plain object from an AssetGraphEntryFileNode message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {AssetGraphEntryFileNode} message AssetGraphEntryFileNode
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetGraphEntryFileNode.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.id = '';
      object.value = null;
    }
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.value != null && message.hasOwnProperty('value'))
      object.value = $root.Entry.toObject(message.value, options);
    if (
      message.correspondingRequest != null &&
      message.hasOwnProperty('correspondingRequest')
    ) {
      object.correspondingRequest = message.correspondingRequest;
      if (options.oneofs) object._correspondingRequest = 'correspondingRequest';
    }
    return object;
  };

  /**
   * Converts this AssetGraphEntryFileNode to JSON.
   * @function toJSON
   * @memberof AssetGraphEntryFileNode
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetGraphEntryFileNode.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetGraphEntryFileNode
   * @function getTypeUrl
   * @memberof AssetGraphEntryFileNode
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetGraphEntryFileNode.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetGraphEntryFileNode';
  };

  return AssetGraphEntryFileNode;
})());

export const AssetGraphRootNode = ($root.AssetGraphRootNode = (() => {
  /**
   * Properties of an AssetGraphRootNode.
   * @exports IAssetGraphRootNode
   * @interface IAssetGraphRootNode
   * @property {string|null} [id] AssetGraphRootNode id
   * @property {string|null} [value] AssetGraphRootNode value
   */

  /**
   * Constructs a new AssetGraphRootNode.
   * @exports AssetGraphRootNode
   * @classdesc Represents an AssetGraphRootNode.
   * @implements IAssetGraphRootNode
   * @constructor
   * @param {IAssetGraphRootNode=} [properties] Properties to set
   */
  function AssetGraphRootNode(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetGraphRootNode id.
   * @member {string} id
   * @memberof AssetGraphRootNode
   * @instance
   */
  AssetGraphRootNode.prototype.id = '';

  /**
   * AssetGraphRootNode value.
   * @member {string|null|undefined} value
   * @memberof AssetGraphRootNode
   * @instance
   */
  AssetGraphRootNode.prototype.value = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(AssetGraphRootNode.prototype, '_value', {
    get: $util.oneOfGetter(($oneOfFields = ['value'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new AssetGraphRootNode instance using the specified properties.
   * @function create
   * @memberof AssetGraphRootNode
   * @static
   * @param {IAssetGraphRootNode=} [properties] Properties to set
   * @returns {AssetGraphRootNode} AssetGraphRootNode instance
   */
  AssetGraphRootNode.create = function create(properties) {
    return new AssetGraphRootNode(properties);
  };

  /**
   * Encodes the specified AssetGraphRootNode message. Does not implicitly {@link AssetGraphRootNode.verify|verify} messages.
   * @function encode
   * @memberof AssetGraphRootNode
   * @static
   * @param {IAssetGraphRootNode} message AssetGraphRootNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphRootNode.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (message.value != null && Object.hasOwnProperty.call(message, 'value'))
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.value);
    return writer;
  };

  /**
   * Encodes the specified AssetGraphRootNode message, length delimited. Does not implicitly {@link AssetGraphRootNode.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetGraphRootNode
   * @static
   * @param {IAssetGraphRootNode} message AssetGraphRootNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphRootNode.encodeDelimited = function encodeDelimited(
    message,
    writer,
  ) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetGraphRootNode message from the specified reader or buffer.
   * @function decode
   * @memberof AssetGraphRootNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetGraphRootNode} AssetGraphRootNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphRootNode.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetGraphRootNode();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.value = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetGraphRootNode message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetGraphRootNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetGraphRootNode} AssetGraphRootNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphRootNode.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetGraphRootNode message.
   * @function verify
   * @memberof AssetGraphRootNode
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetGraphRootNode.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.value != null && message.hasOwnProperty('value')) {
      properties._value = 1;
      if (!$util.isString(message.value)) return 'value: string expected';
    }
    return null;
  };

  /**
   * Creates an AssetGraphRootNode message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetGraphRootNode
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetGraphRootNode} AssetGraphRootNode
   */
  AssetGraphRootNode.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetGraphRootNode) return object;
    let message = new $root.AssetGraphRootNode();
    if (object.id != null) message.id = String(object.id);
    if (object.value != null) message.value = String(object.value);
    return message;
  };

  /**
   * Creates a plain object from an AssetGraphRootNode message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetGraphRootNode
   * @static
   * @param {AssetGraphRootNode} message AssetGraphRootNode
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetGraphRootNode.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) object.id = '';
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.value != null && message.hasOwnProperty('value')) {
      object.value = message.value;
      if (options.oneofs) object._value = 'value';
    }
    return object;
  };

  /**
   * Converts this AssetGraphRootNode to JSON.
   * @function toJSON
   * @memberof AssetGraphRootNode
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetGraphRootNode.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetGraphRootNode
   * @function getTypeUrl
   * @memberof AssetGraphRootNode
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetGraphRootNode.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetGraphRootNode';
  };

  return AssetGraphRootNode;
})());

export const AssetGraphDependencyNode = ($root.AssetGraphDependencyNode =
  (() => {
    /**
     * Properties of an AssetGraphDependencyNode.
     * @exports IAssetGraphDependencyNode
     * @interface IAssetGraphDependencyNode
     * @property {string|null} [id] AssetGraphDependencyNode id
     * @property {IDependency|null} [value] AssetGraphDependencyNode value
     * @property {boolean|null} [complete] AssetGraphDependencyNode complete
     * @property {string|null} [correspondingRequest] AssetGraphDependencyNode correspondingRequest
     * @property {boolean|null} [deferred] AssetGraphDependencyNode deferred
     * @property {boolean|null} [hasDeferred] AssetGraphDependencyNode hasDeferred
     * @property {Array.<string>|null} [usedSymbolsDown] AssetGraphDependencyNode usedSymbolsDown
     * @property {string|null} [usedSymbolsUp] AssetGraphDependencyNode usedSymbolsUp
     * @property {boolean|null} [usedSymbolsDownDirty] AssetGraphDependencyNode usedSymbolsDownDirty
     * @property {boolean|null} [usedSymbolsUpDirtyDown] AssetGraphDependencyNode usedSymbolsUpDirtyDown
     * @property {boolean|null} [usedSymbolsUpDirtyUp] AssetGraphDependencyNode usedSymbolsUpDirtyUp
     * @property {boolean|null} [excluded] AssetGraphDependencyNode excluded
     */

    /**
     * Constructs a new AssetGraphDependencyNode.
     * @exports AssetGraphDependencyNode
     * @classdesc Represents an AssetGraphDependencyNode.
     * @implements IAssetGraphDependencyNode
     * @constructor
     * @param {IAssetGraphDependencyNode=} [properties] Properties to set
     */
    function AssetGraphDependencyNode(properties) {
      this.usedSymbolsDown = [];
      if (properties)
        for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
          if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
    }

    /**
     * AssetGraphDependencyNode id.
     * @member {string} id
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.id = '';

    /**
     * AssetGraphDependencyNode value.
     * @member {IDependency|null|undefined} value
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.value = null;

    /**
     * AssetGraphDependencyNode complete.
     * @member {boolean|null|undefined} complete
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.complete = null;

    /**
     * AssetGraphDependencyNode correspondingRequest.
     * @member {string|null|undefined} correspondingRequest
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.correspondingRequest = null;

    /**
     * AssetGraphDependencyNode deferred.
     * @member {boolean} deferred
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.deferred = false;

    /**
     * AssetGraphDependencyNode hasDeferred.
     * @member {boolean|null|undefined} hasDeferred
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.hasDeferred = null;

    /**
     * AssetGraphDependencyNode usedSymbolsDown.
     * @member {Array.<string>} usedSymbolsDown
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.usedSymbolsDown = $util.emptyArray;

    /**
     * AssetGraphDependencyNode usedSymbolsUp.
     * @member {string} usedSymbolsUp
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.usedSymbolsUp = '';

    /**
     * AssetGraphDependencyNode usedSymbolsDownDirty.
     * @member {boolean} usedSymbolsDownDirty
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.usedSymbolsDownDirty = false;

    /**
     * AssetGraphDependencyNode usedSymbolsUpDirtyDown.
     * @member {boolean} usedSymbolsUpDirtyDown
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.usedSymbolsUpDirtyDown = false;

    /**
     * AssetGraphDependencyNode usedSymbolsUpDirtyUp.
     * @member {boolean} usedSymbolsUpDirtyUp
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.usedSymbolsUpDirtyUp = false;

    /**
     * AssetGraphDependencyNode excluded.
     * @member {boolean} excluded
     * @memberof AssetGraphDependencyNode
     * @instance
     */
    AssetGraphDependencyNode.prototype.excluded = false;

    // OneOf field names bound to virtual getters and setters
    let $oneOfFields;

    // Virtual OneOf for proto3 optional field
    Object.defineProperty(AssetGraphDependencyNode.prototype, '_complete', {
      get: $util.oneOfGetter(($oneOfFields = ['complete'])),
      set: $util.oneOfSetter($oneOfFields),
    });

    // Virtual OneOf for proto3 optional field
    Object.defineProperty(
      AssetGraphDependencyNode.prototype,
      '_correspondingRequest',
      {
        get: $util.oneOfGetter(($oneOfFields = ['correspondingRequest'])),
        set: $util.oneOfSetter($oneOfFields),
      },
    );

    // Virtual OneOf for proto3 optional field
    Object.defineProperty(AssetGraphDependencyNode.prototype, '_hasDeferred', {
      get: $util.oneOfGetter(($oneOfFields = ['hasDeferred'])),
      set: $util.oneOfSetter($oneOfFields),
    });

    /**
     * Creates a new AssetGraphDependencyNode instance using the specified properties.
     * @function create
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {IAssetGraphDependencyNode=} [properties] Properties to set
     * @returns {AssetGraphDependencyNode} AssetGraphDependencyNode instance
     */
    AssetGraphDependencyNode.create = function create(properties) {
      return new AssetGraphDependencyNode(properties);
    };

    /**
     * Encodes the specified AssetGraphDependencyNode message. Does not implicitly {@link AssetGraphDependencyNode.verify|verify} messages.
     * @function encode
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {IAssetGraphDependencyNode} message AssetGraphDependencyNode message or plain object to encode
     * @param {$protobuf.Writer} [writer] Writer to encode to
     * @returns {$protobuf.Writer} Writer
     */
    AssetGraphDependencyNode.encode = function encode(message, writer) {
      if (!writer) writer = $Writer.create();
      if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
        writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
      if (message.value != null && Object.hasOwnProperty.call(message, 'value'))
        $root.Dependency.encode(
          message.value,
          writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
        ).ldelim();
      if (
        message.complete != null &&
        Object.hasOwnProperty.call(message, 'complete')
      )
        writer.uint32(/* id 3, wireType 0 =*/ 24).bool(message.complete);
      if (
        message.correspondingRequest != null &&
        Object.hasOwnProperty.call(message, 'correspondingRequest')
      )
        writer
          .uint32(/* id 4, wireType 2 =*/ 34)
          .string(message.correspondingRequest);
      if (
        message.deferred != null &&
        Object.hasOwnProperty.call(message, 'deferred')
      )
        writer.uint32(/* id 5, wireType 0 =*/ 40).bool(message.deferred);
      if (
        message.hasDeferred != null &&
        Object.hasOwnProperty.call(message, 'hasDeferred')
      )
        writer.uint32(/* id 6, wireType 0 =*/ 48).bool(message.hasDeferred);
      if (message.usedSymbolsDown != null && message.usedSymbolsDown.length)
        for (let i = 0; i < message.usedSymbolsDown.length; ++i)
          writer
            .uint32(/* id 7, wireType 2 =*/ 58)
            .string(message.usedSymbolsDown[i]);
      if (
        message.usedSymbolsUp != null &&
        Object.hasOwnProperty.call(message, 'usedSymbolsUp')
      )
        writer.uint32(/* id 8, wireType 2 =*/ 66).string(message.usedSymbolsUp);
      if (
        message.usedSymbolsDownDirty != null &&
        Object.hasOwnProperty.call(message, 'usedSymbolsDownDirty')
      )
        writer
          .uint32(/* id 9, wireType 0 =*/ 72)
          .bool(message.usedSymbolsDownDirty);
      if (
        message.usedSymbolsUpDirtyDown != null &&
        Object.hasOwnProperty.call(message, 'usedSymbolsUpDirtyDown')
      )
        writer
          .uint32(/* id 10, wireType 0 =*/ 80)
          .bool(message.usedSymbolsUpDirtyDown);
      if (
        message.usedSymbolsUpDirtyUp != null &&
        Object.hasOwnProperty.call(message, 'usedSymbolsUpDirtyUp')
      )
        writer
          .uint32(/* id 11, wireType 0 =*/ 88)
          .bool(message.usedSymbolsUpDirtyUp);
      if (
        message.excluded != null &&
        Object.hasOwnProperty.call(message, 'excluded')
      )
        writer.uint32(/* id 12, wireType 0 =*/ 96).bool(message.excluded);
      return writer;
    };

    /**
     * Encodes the specified AssetGraphDependencyNode message, length delimited. Does not implicitly {@link AssetGraphDependencyNode.verify|verify} messages.
     * @function encodeDelimited
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {IAssetGraphDependencyNode} message AssetGraphDependencyNode message or plain object to encode
     * @param {$protobuf.Writer} [writer] Writer to encode to
     * @returns {$protobuf.Writer} Writer
     */
    AssetGraphDependencyNode.encodeDelimited = function encodeDelimited(
      message,
      writer,
    ) {
      return this.encode(message, writer).ldelim();
    };

    /**
     * Decodes an AssetGraphDependencyNode message from the specified reader or buffer.
     * @function decode
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
     * @param {number} [length] Message length if known beforehand
     * @returns {AssetGraphDependencyNode} AssetGraphDependencyNode
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    AssetGraphDependencyNode.decode = function decode(reader, length) {
      if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
      let end = length === undefined ? reader.len : reader.pos + length,
        message = new $root.AssetGraphDependencyNode();
      while (reader.pos < end) {
        let tag = reader.uint32();
        switch (tag >>> 3) {
          case 1: {
            message.id = reader.string();
            break;
          }
          case 2: {
            message.value = $root.Dependency.decode(reader, reader.uint32());
            break;
          }
          case 3: {
            message.complete = reader.bool();
            break;
          }
          case 4: {
            message.correspondingRequest = reader.string();
            break;
          }
          case 5: {
            message.deferred = reader.bool();
            break;
          }
          case 6: {
            message.hasDeferred = reader.bool();
            break;
          }
          case 7: {
            if (!(message.usedSymbolsDown && message.usedSymbolsDown.length))
              message.usedSymbolsDown = [];
            message.usedSymbolsDown.push(reader.string());
            break;
          }
          case 8: {
            message.usedSymbolsUp = reader.string();
            break;
          }
          case 9: {
            message.usedSymbolsDownDirty = reader.bool();
            break;
          }
          case 10: {
            message.usedSymbolsUpDirtyDown = reader.bool();
            break;
          }
          case 11: {
            message.usedSymbolsUpDirtyUp = reader.bool();
            break;
          }
          case 12: {
            message.excluded = reader.bool();
            break;
          }
          default:
            reader.skipType(tag & 7);
            break;
        }
      }
      return message;
    };

    /**
     * Decodes an AssetGraphDependencyNode message from the specified reader or buffer, length delimited.
     * @function decodeDelimited
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
     * @returns {AssetGraphDependencyNode} AssetGraphDependencyNode
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    AssetGraphDependencyNode.decodeDelimited = function decodeDelimited(
      reader,
    ) {
      if (!(reader instanceof $Reader)) reader = new $Reader(reader);
      return this.decode(reader, reader.uint32());
    };

    /**
     * Verifies an AssetGraphDependencyNode message.
     * @function verify
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {Object.<string,*>} message Plain object to verify
     * @returns {string|null} `null` if valid, otherwise the reason why it is not
     */
    AssetGraphDependencyNode.verify = function verify(message) {
      if (typeof message !== 'object' || message === null)
        return 'object expected';
      let properties = {};
      if (message.id != null && message.hasOwnProperty('id'))
        if (!$util.isString(message.id)) return 'id: string expected';
      if (message.value != null && message.hasOwnProperty('value')) {
        let error = $root.Dependency.verify(message.value);
        if (error) return 'value.' + error;
      }
      if (message.complete != null && message.hasOwnProperty('complete')) {
        properties._complete = 1;
        if (typeof message.complete !== 'boolean')
          return 'complete: boolean expected';
      }
      if (
        message.correspondingRequest != null &&
        message.hasOwnProperty('correspondingRequest')
      ) {
        properties._correspondingRequest = 1;
        if (!$util.isString(message.correspondingRequest))
          return 'correspondingRequest: string expected';
      }
      if (message.deferred != null && message.hasOwnProperty('deferred'))
        if (typeof message.deferred !== 'boolean')
          return 'deferred: boolean expected';
      if (
        message.hasDeferred != null &&
        message.hasOwnProperty('hasDeferred')
      ) {
        properties._hasDeferred = 1;
        if (typeof message.hasDeferred !== 'boolean')
          return 'hasDeferred: boolean expected';
      }
      if (
        message.usedSymbolsDown != null &&
        message.hasOwnProperty('usedSymbolsDown')
      ) {
        if (!Array.isArray(message.usedSymbolsDown))
          return 'usedSymbolsDown: array expected';
        for (let i = 0; i < message.usedSymbolsDown.length; ++i)
          if (!$util.isString(message.usedSymbolsDown[i]))
            return 'usedSymbolsDown: string[] expected';
      }
      if (
        message.usedSymbolsUp != null &&
        message.hasOwnProperty('usedSymbolsUp')
      )
        if (!$util.isString(message.usedSymbolsUp))
          return 'usedSymbolsUp: string expected';
      if (
        message.usedSymbolsDownDirty != null &&
        message.hasOwnProperty('usedSymbolsDownDirty')
      )
        if (typeof message.usedSymbolsDownDirty !== 'boolean')
          return 'usedSymbolsDownDirty: boolean expected';
      if (
        message.usedSymbolsUpDirtyDown != null &&
        message.hasOwnProperty('usedSymbolsUpDirtyDown')
      )
        if (typeof message.usedSymbolsUpDirtyDown !== 'boolean')
          return 'usedSymbolsUpDirtyDown: boolean expected';
      if (
        message.usedSymbolsUpDirtyUp != null &&
        message.hasOwnProperty('usedSymbolsUpDirtyUp')
      )
        if (typeof message.usedSymbolsUpDirtyUp !== 'boolean')
          return 'usedSymbolsUpDirtyUp: boolean expected';
      if (message.excluded != null && message.hasOwnProperty('excluded'))
        if (typeof message.excluded !== 'boolean')
          return 'excluded: boolean expected';
      return null;
    };

    /**
     * Creates an AssetGraphDependencyNode message from a plain object. Also converts values to their respective internal types.
     * @function fromObject
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {Object.<string,*>} object Plain object
     * @returns {AssetGraphDependencyNode} AssetGraphDependencyNode
     */
    AssetGraphDependencyNode.fromObject = function fromObject(object) {
      if (object instanceof $root.AssetGraphDependencyNode) return object;
      let message = new $root.AssetGraphDependencyNode();
      if (object.id != null) message.id = String(object.id);
      if (object.value != null) {
        if (typeof object.value !== 'object')
          throw TypeError('.AssetGraphDependencyNode.value: object expected');
        message.value = $root.Dependency.fromObject(object.value);
      }
      if (object.complete != null) message.complete = Boolean(object.complete);
      if (object.correspondingRequest != null)
        message.correspondingRequest = String(object.correspondingRequest);
      if (object.deferred != null) message.deferred = Boolean(object.deferred);
      if (object.hasDeferred != null)
        message.hasDeferred = Boolean(object.hasDeferred);
      if (object.usedSymbolsDown) {
        if (!Array.isArray(object.usedSymbolsDown))
          throw TypeError(
            '.AssetGraphDependencyNode.usedSymbolsDown: array expected',
          );
        message.usedSymbolsDown = [];
        for (let i = 0; i < object.usedSymbolsDown.length; ++i)
          message.usedSymbolsDown[i] = String(object.usedSymbolsDown[i]);
      }
      if (object.usedSymbolsUp != null)
        message.usedSymbolsUp = String(object.usedSymbolsUp);
      if (object.usedSymbolsDownDirty != null)
        message.usedSymbolsDownDirty = Boolean(object.usedSymbolsDownDirty);
      if (object.usedSymbolsUpDirtyDown != null)
        message.usedSymbolsUpDirtyDown = Boolean(object.usedSymbolsUpDirtyDown);
      if (object.usedSymbolsUpDirtyUp != null)
        message.usedSymbolsUpDirtyUp = Boolean(object.usedSymbolsUpDirtyUp);
      if (object.excluded != null) message.excluded = Boolean(object.excluded);
      return message;
    };

    /**
     * Creates a plain object from an AssetGraphDependencyNode message. Also converts values to other types if specified.
     * @function toObject
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {AssetGraphDependencyNode} message AssetGraphDependencyNode
     * @param {$protobuf.IConversionOptions} [options] Conversion options
     * @returns {Object.<string,*>} Plain object
     */
    AssetGraphDependencyNode.toObject = function toObject(message, options) {
      if (!options) options = {};
      let object = {};
      if (options.arrays || options.defaults) object.usedSymbolsDown = [];
      if (options.defaults) {
        object.id = '';
        object.value = null;
        object.deferred = false;
        object.usedSymbolsUp = '';
        object.usedSymbolsDownDirty = false;
        object.usedSymbolsUpDirtyDown = false;
        object.usedSymbolsUpDirtyUp = false;
        object.excluded = false;
      }
      if (message.id != null && message.hasOwnProperty('id'))
        object.id = message.id;
      if (message.value != null && message.hasOwnProperty('value'))
        object.value = $root.Dependency.toObject(message.value, options);
      if (message.complete != null && message.hasOwnProperty('complete')) {
        object.complete = message.complete;
        if (options.oneofs) object._complete = 'complete';
      }
      if (
        message.correspondingRequest != null &&
        message.hasOwnProperty('correspondingRequest')
      ) {
        object.correspondingRequest = message.correspondingRequest;
        if (options.oneofs)
          object._correspondingRequest = 'correspondingRequest';
      }
      if (message.deferred != null && message.hasOwnProperty('deferred'))
        object.deferred = message.deferred;
      if (
        message.hasDeferred != null &&
        message.hasOwnProperty('hasDeferred')
      ) {
        object.hasDeferred = message.hasDeferred;
        if (options.oneofs) object._hasDeferred = 'hasDeferred';
      }
      if (message.usedSymbolsDown && message.usedSymbolsDown.length) {
        object.usedSymbolsDown = [];
        for (let j = 0; j < message.usedSymbolsDown.length; ++j)
          object.usedSymbolsDown[j] = message.usedSymbolsDown[j];
      }
      if (
        message.usedSymbolsUp != null &&
        message.hasOwnProperty('usedSymbolsUp')
      )
        object.usedSymbolsUp = message.usedSymbolsUp;
      if (
        message.usedSymbolsDownDirty != null &&
        message.hasOwnProperty('usedSymbolsDownDirty')
      )
        object.usedSymbolsDownDirty = message.usedSymbolsDownDirty;
      if (
        message.usedSymbolsUpDirtyDown != null &&
        message.hasOwnProperty('usedSymbolsUpDirtyDown')
      )
        object.usedSymbolsUpDirtyDown = message.usedSymbolsUpDirtyDown;
      if (
        message.usedSymbolsUpDirtyUp != null &&
        message.hasOwnProperty('usedSymbolsUpDirtyUp')
      )
        object.usedSymbolsUpDirtyUp = message.usedSymbolsUpDirtyUp;
      if (message.excluded != null && message.hasOwnProperty('excluded'))
        object.excluded = message.excluded;
      return object;
    };

    /**
     * Converts this AssetGraphDependencyNode to JSON.
     * @function toJSON
     * @memberof AssetGraphDependencyNode
     * @instance
     * @returns {Object.<string,*>} JSON object
     */
    AssetGraphDependencyNode.prototype.toJSON = function toJSON() {
      return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
    };

    /**
     * Gets the default type url for AssetGraphDependencyNode
     * @function getTypeUrl
     * @memberof AssetGraphDependencyNode
     * @static
     * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
     * @returns {string} The default type url
     */
    AssetGraphDependencyNode.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
      if (typeUrlPrefix === undefined) {
        typeUrlPrefix = 'type.googleapis.com';
      }
      return typeUrlPrefix + '/AssetGraphDependencyNode';
    };

    return AssetGraphDependencyNode;
  })());

export const Asset = ($root.Asset = (() => {
  /**
   * Properties of an Asset.
   * @exports IAsset
   * @interface IAsset
   * @property {string|null} [id] Asset id
   * @property {boolean|null} [committed] Asset committed
   * @property {string|null} [filePath] Asset filePath
   * @property {string|null} [query] Asset query
   * @property {string|null} [type] Asset type
   * @property {Object.<string,IDependency>|null} [dependencies] Asset dependencies
   * @property {BundleBehavior|null} [bundleBehavior] Asset bundleBehavior
   * @property {boolean|null} [isBundleSplittable] Asset isBundleSplittable
   * @property {boolean|null} [isSource] Asset isSource
   * @property {string|null} [env] Asset env
   * @property {string|null} [meta] Asset meta
   * @property {string|null} [stats] Asset stats
   * @property {string|null} [contentKey] Asset contentKey
   * @property {string|null} [mapKey] Asset mapKey
   * @property {string|null} [outputHash] Asset outputHash
   * @property {string|null} [pipeline] Asset pipeline
   * @property {string|null} [astKey] Asset astKey
   * @property {IASTGenerator|null} [astGenerator] Asset astGenerator
   * @property {Object.<string,IAssetSymbol>|null} [symbols] Asset symbols
   * @property {boolean|null} [sideEffects] Asset sideEffects
   * @property {string|null} [uniqueKey] Asset uniqueKey
   * @property {string|null} [configPath] Asset configPath
   * @property {string|null} [plugin] Asset plugin
   * @property {string|null} [configKeyPath] Asset configKeyPath
   * @property {boolean|null} [isLargeBlob] Asset isLargeBlob
   */

  /**
   * Constructs a new Asset.
   * @exports Asset
   * @classdesc Represents an Asset.
   * @implements IAsset
   * @constructor
   * @param {IAsset=} [properties] Properties to set
   */
  function Asset(properties) {
    this.dependencies = {};
    this.symbols = {};
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * Asset id.
   * @member {string} id
   * @memberof Asset
   * @instance
   */
  Asset.prototype.id = '';

  /**
   * Asset committed.
   * @member {boolean} committed
   * @memberof Asset
   * @instance
   */
  Asset.prototype.committed = false;

  /**
   * Asset filePath.
   * @member {string} filePath
   * @memberof Asset
   * @instance
   */
  Asset.prototype.filePath = '';

  /**
   * Asset query.
   * @member {string|null|undefined} query
   * @memberof Asset
   * @instance
   */
  Asset.prototype.query = null;

  /**
   * Asset type.
   * @member {string} type
   * @memberof Asset
   * @instance
   */
  Asset.prototype.type = '';

  /**
   * Asset dependencies.
   * @member {Object.<string,IDependency>} dependencies
   * @memberof Asset
   * @instance
   */
  Asset.prototype.dependencies = $util.emptyObject;

  /**
   * Asset bundleBehavior.
   * @member {BundleBehavior} bundleBehavior
   * @memberof Asset
   * @instance
   */
  Asset.prototype.bundleBehavior = 0;

  /**
   * Asset isBundleSplittable.
   * @member {boolean} isBundleSplittable
   * @memberof Asset
   * @instance
   */
  Asset.prototype.isBundleSplittable = false;

  /**
   * Asset isSource.
   * @member {boolean} isSource
   * @memberof Asset
   * @instance
   */
  Asset.prototype.isSource = false;

  /**
   * Asset env.
   * @member {string} env
   * @memberof Asset
   * @instance
   */
  Asset.prototype.env = '';

  /**
   * Asset meta.
   * @member {string} meta
   * @memberof Asset
   * @instance
   */
  Asset.prototype.meta = '';

  /**
   * Asset stats.
   * @member {string} stats
   * @memberof Asset
   * @instance
   */
  Asset.prototype.stats = '';

  /**
   * Asset contentKey.
   * @member {string|null|undefined} contentKey
   * @memberof Asset
   * @instance
   */
  Asset.prototype.contentKey = null;

  /**
   * Asset mapKey.
   * @member {string|null|undefined} mapKey
   * @memberof Asset
   * @instance
   */
  Asset.prototype.mapKey = null;

  /**
   * Asset outputHash.
   * @member {string|null|undefined} outputHash
   * @memberof Asset
   * @instance
   */
  Asset.prototype.outputHash = null;

  /**
   * Asset pipeline.
   * @member {string|null|undefined} pipeline
   * @memberof Asset
   * @instance
   */
  Asset.prototype.pipeline = null;

  /**
   * Asset astKey.
   * @member {string|null|undefined} astKey
   * @memberof Asset
   * @instance
   */
  Asset.prototype.astKey = null;

  /**
   * Asset astGenerator.
   * @member {IASTGenerator|null|undefined} astGenerator
   * @memberof Asset
   * @instance
   */
  Asset.prototype.astGenerator = null;

  /**
   * Asset symbols.
   * @member {Object.<string,IAssetSymbol>} symbols
   * @memberof Asset
   * @instance
   */
  Asset.prototype.symbols = $util.emptyObject;

  /**
   * Asset sideEffects.
   * @member {boolean} sideEffects
   * @memberof Asset
   * @instance
   */
  Asset.prototype.sideEffects = false;

  /**
   * Asset uniqueKey.
   * @member {string|null|undefined} uniqueKey
   * @memberof Asset
   * @instance
   */
  Asset.prototype.uniqueKey = null;

  /**
   * Asset configPath.
   * @member {string|null|undefined} configPath
   * @memberof Asset
   * @instance
   */
  Asset.prototype.configPath = null;

  /**
   * Asset plugin.
   * @member {string|null|undefined} plugin
   * @memberof Asset
   * @instance
   */
  Asset.prototype.plugin = null;

  /**
   * Asset configKeyPath.
   * @member {string|null|undefined} configKeyPath
   * @memberof Asset
   * @instance
   */
  Asset.prototype.configKeyPath = null;

  /**
   * Asset isLargeBlob.
   * @member {boolean|null|undefined} isLargeBlob
   * @memberof Asset
   * @instance
   */
  Asset.prototype.isLargeBlob = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_query', {
    get: $util.oneOfGetter(($oneOfFields = ['query'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_contentKey', {
    get: $util.oneOfGetter(($oneOfFields = ['contentKey'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_mapKey', {
    get: $util.oneOfGetter(($oneOfFields = ['mapKey'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_outputHash', {
    get: $util.oneOfGetter(($oneOfFields = ['outputHash'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_pipeline', {
    get: $util.oneOfGetter(($oneOfFields = ['pipeline'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_astKey', {
    get: $util.oneOfGetter(($oneOfFields = ['astKey'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_astGenerator', {
    get: $util.oneOfGetter(($oneOfFields = ['astGenerator'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_uniqueKey', {
    get: $util.oneOfGetter(($oneOfFields = ['uniqueKey'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_configPath', {
    get: $util.oneOfGetter(($oneOfFields = ['configPath'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_plugin', {
    get: $util.oneOfGetter(($oneOfFields = ['plugin'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_configKeyPath', {
    get: $util.oneOfGetter(($oneOfFields = ['configKeyPath'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Asset.prototype, '_isLargeBlob', {
    get: $util.oneOfGetter(($oneOfFields = ['isLargeBlob'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new Asset instance using the specified properties.
   * @function create
   * @memberof Asset
   * @static
   * @param {IAsset=} [properties] Properties to set
   * @returns {Asset} Asset instance
   */
  Asset.create = function create(properties) {
    return new Asset(properties);
  };

  /**
   * Encodes the specified Asset message. Does not implicitly {@link Asset.verify|verify} messages.
   * @function encode
   * @memberof Asset
   * @static
   * @param {IAsset} message Asset message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Asset.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (
      message.committed != null &&
      Object.hasOwnProperty.call(message, 'committed')
    )
      writer.uint32(/* id 2, wireType 0 =*/ 16).bool(message.committed);
    if (
      message.filePath != null &&
      Object.hasOwnProperty.call(message, 'filePath')
    )
      writer.uint32(/* id 3, wireType 2 =*/ 26).string(message.filePath);
    if (message.query != null && Object.hasOwnProperty.call(message, 'query'))
      writer.uint32(/* id 4, wireType 2 =*/ 34).string(message.query);
    if (message.type != null && Object.hasOwnProperty.call(message, 'type'))
      writer.uint32(/* id 5, wireType 2 =*/ 42).string(message.type);
    if (
      message.dependencies != null &&
      Object.hasOwnProperty.call(message, 'dependencies')
    )
      for (
        let keys = Object.keys(message.dependencies), i = 0;
        i < keys.length;
        ++i
      ) {
        writer
          .uint32(/* id 6, wireType 2 =*/ 50)
          .fork()
          .uint32(/* id 1, wireType 2 =*/ 10)
          .string(keys[i]);
        $root.Dependency.encode(
          message.dependencies[keys[i]],
          writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
        )
          .ldelim()
          .ldelim();
      }
    if (
      message.bundleBehavior != null &&
      Object.hasOwnProperty.call(message, 'bundleBehavior')
    )
      writer.uint32(/* id 7, wireType 0 =*/ 56).int32(message.bundleBehavior);
    if (
      message.isBundleSplittable != null &&
      Object.hasOwnProperty.call(message, 'isBundleSplittable')
    )
      writer
        .uint32(/* id 8, wireType 0 =*/ 64)
        .bool(message.isBundleSplittable);
    if (
      message.isSource != null &&
      Object.hasOwnProperty.call(message, 'isSource')
    )
      writer.uint32(/* id 9, wireType 0 =*/ 72).bool(message.isSource);
    if (message.env != null && Object.hasOwnProperty.call(message, 'env'))
      writer.uint32(/* id 10, wireType 2 =*/ 82).string(message.env);
    if (message.meta != null && Object.hasOwnProperty.call(message, 'meta'))
      writer.uint32(/* id 11, wireType 2 =*/ 90).string(message.meta);
    if (message.stats != null && Object.hasOwnProperty.call(message, 'stats'))
      writer.uint32(/* id 12, wireType 2 =*/ 98).string(message.stats);
    if (
      message.contentKey != null &&
      Object.hasOwnProperty.call(message, 'contentKey')
    )
      writer.uint32(/* id 13, wireType 2 =*/ 106).string(message.contentKey);
    if (message.mapKey != null && Object.hasOwnProperty.call(message, 'mapKey'))
      writer.uint32(/* id 14, wireType 2 =*/ 114).string(message.mapKey);
    if (
      message.outputHash != null &&
      Object.hasOwnProperty.call(message, 'outputHash')
    )
      writer.uint32(/* id 15, wireType 2 =*/ 122).string(message.outputHash);
    if (
      message.pipeline != null &&
      Object.hasOwnProperty.call(message, 'pipeline')
    )
      writer.uint32(/* id 16, wireType 2 =*/ 130).string(message.pipeline);
    if (message.astKey != null && Object.hasOwnProperty.call(message, 'astKey'))
      writer.uint32(/* id 17, wireType 2 =*/ 138).string(message.astKey);
    if (
      message.astGenerator != null &&
      Object.hasOwnProperty.call(message, 'astGenerator')
    )
      $root.ASTGenerator.encode(
        message.astGenerator,
        writer.uint32(/* id 18, wireType 2 =*/ 146).fork(),
      ).ldelim();
    if (
      message.symbols != null &&
      Object.hasOwnProperty.call(message, 'symbols')
    )
      for (
        let keys = Object.keys(message.symbols), i = 0;
        i < keys.length;
        ++i
      ) {
        writer
          .uint32(/* id 19, wireType 2 =*/ 154)
          .fork()
          .uint32(/* id 1, wireType 2 =*/ 10)
          .string(keys[i]);
        $root.AssetSymbol.encode(
          message.symbols[keys[i]],
          writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
        )
          .ldelim()
          .ldelim();
      }
    if (
      message.sideEffects != null &&
      Object.hasOwnProperty.call(message, 'sideEffects')
    )
      writer.uint32(/* id 20, wireType 0 =*/ 160).bool(message.sideEffects);
    if (
      message.uniqueKey != null &&
      Object.hasOwnProperty.call(message, 'uniqueKey')
    )
      writer.uint32(/* id 21, wireType 2 =*/ 170).string(message.uniqueKey);
    if (
      message.configPath != null &&
      Object.hasOwnProperty.call(message, 'configPath')
    )
      writer.uint32(/* id 22, wireType 2 =*/ 178).string(message.configPath);
    if (message.plugin != null && Object.hasOwnProperty.call(message, 'plugin'))
      writer.uint32(/* id 23, wireType 2 =*/ 186).string(message.plugin);
    if (
      message.configKeyPath != null &&
      Object.hasOwnProperty.call(message, 'configKeyPath')
    )
      writer.uint32(/* id 24, wireType 2 =*/ 194).string(message.configKeyPath);
    if (
      message.isLargeBlob != null &&
      Object.hasOwnProperty.call(message, 'isLargeBlob')
    )
      writer.uint32(/* id 25, wireType 0 =*/ 200).bool(message.isLargeBlob);
    return writer;
  };

  /**
   * Encodes the specified Asset message, length delimited. Does not implicitly {@link Asset.verify|verify} messages.
   * @function encodeDelimited
   * @memberof Asset
   * @static
   * @param {IAsset} message Asset message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Asset.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an Asset message from the specified reader or buffer.
   * @function decode
   * @memberof Asset
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {Asset} Asset
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Asset.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.Asset(),
      key,
      value;
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.committed = reader.bool();
          break;
        }
        case 3: {
          message.filePath = reader.string();
          break;
        }
        case 4: {
          message.query = reader.string();
          break;
        }
        case 5: {
          message.type = reader.string();
          break;
        }
        case 6: {
          if (message.dependencies === $util.emptyObject)
            message.dependencies = {};
          let end2 = reader.uint32() + reader.pos;
          key = '';
          value = null;
          while (reader.pos < end2) {
            let tag2 = reader.uint32();
            switch (tag2 >>> 3) {
              case 1:
                key = reader.string();
                break;
              case 2:
                value = $root.Dependency.decode(reader, reader.uint32());
                break;
              default:
                reader.skipType(tag2 & 7);
                break;
            }
          }
          message.dependencies[key] = value;
          break;
        }
        case 7: {
          message.bundleBehavior = reader.int32();
          break;
        }
        case 8: {
          message.isBundleSplittable = reader.bool();
          break;
        }
        case 9: {
          message.isSource = reader.bool();
          break;
        }
        case 10: {
          message.env = reader.string();
          break;
        }
        case 11: {
          message.meta = reader.string();
          break;
        }
        case 12: {
          message.stats = reader.string();
          break;
        }
        case 13: {
          message.contentKey = reader.string();
          break;
        }
        case 14: {
          message.mapKey = reader.string();
          break;
        }
        case 15: {
          message.outputHash = reader.string();
          break;
        }
        case 16: {
          message.pipeline = reader.string();
          break;
        }
        case 17: {
          message.astKey = reader.string();
          break;
        }
        case 18: {
          message.astGenerator = $root.ASTGenerator.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        case 19: {
          if (message.symbols === $util.emptyObject) message.symbols = {};
          let end2 = reader.uint32() + reader.pos;
          key = '';
          value = null;
          while (reader.pos < end2) {
            let tag2 = reader.uint32();
            switch (tag2 >>> 3) {
              case 1:
                key = reader.string();
                break;
              case 2:
                value = $root.AssetSymbol.decode(reader, reader.uint32());
                break;
              default:
                reader.skipType(tag2 & 7);
                break;
            }
          }
          message.symbols[key] = value;
          break;
        }
        case 20: {
          message.sideEffects = reader.bool();
          break;
        }
        case 21: {
          message.uniqueKey = reader.string();
          break;
        }
        case 22: {
          message.configPath = reader.string();
          break;
        }
        case 23: {
          message.plugin = reader.string();
          break;
        }
        case 24: {
          message.configKeyPath = reader.string();
          break;
        }
        case 25: {
          message.isLargeBlob = reader.bool();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an Asset message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof Asset
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {Asset} Asset
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Asset.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an Asset message.
   * @function verify
   * @memberof Asset
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  Asset.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.committed != null && message.hasOwnProperty('committed'))
      if (typeof message.committed !== 'boolean')
        return 'committed: boolean expected';
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      if (!$util.isString(message.filePath)) return 'filePath: string expected';
    if (message.query != null && message.hasOwnProperty('query')) {
      properties._query = 1;
      if (!$util.isString(message.query)) return 'query: string expected';
    }
    if (message.type != null && message.hasOwnProperty('type'))
      if (!$util.isString(message.type)) return 'type: string expected';
    if (
      message.dependencies != null &&
      message.hasOwnProperty('dependencies')
    ) {
      if (!$util.isObject(message.dependencies))
        return 'dependencies: object expected';
      let key = Object.keys(message.dependencies);
      for (let i = 0; i < key.length; ++i) {
        let error = $root.Dependency.verify(message.dependencies[key[i]]);
        if (error) return 'dependencies.' + error;
      }
    }
    if (
      message.bundleBehavior != null &&
      message.hasOwnProperty('bundleBehavior')
    )
      switch (message.bundleBehavior) {
        default:
          return 'bundleBehavior: enum value expected';
        case 0:
        case 1:
          break;
      }
    if (
      message.isBundleSplittable != null &&
      message.hasOwnProperty('isBundleSplittable')
    )
      if (typeof message.isBundleSplittable !== 'boolean')
        return 'isBundleSplittable: boolean expected';
    if (message.isSource != null && message.hasOwnProperty('isSource'))
      if (typeof message.isSource !== 'boolean')
        return 'isSource: boolean expected';
    if (message.env != null && message.hasOwnProperty('env'))
      if (!$util.isString(message.env)) return 'env: string expected';
    if (message.meta != null && message.hasOwnProperty('meta'))
      if (!$util.isString(message.meta)) return 'meta: string expected';
    if (message.stats != null && message.hasOwnProperty('stats'))
      if (!$util.isString(message.stats)) return 'stats: string expected';
    if (message.contentKey != null && message.hasOwnProperty('contentKey')) {
      properties._contentKey = 1;
      if (!$util.isString(message.contentKey))
        return 'contentKey: string expected';
    }
    if (message.mapKey != null && message.hasOwnProperty('mapKey')) {
      properties._mapKey = 1;
      if (!$util.isString(message.mapKey)) return 'mapKey: string expected';
    }
    if (message.outputHash != null && message.hasOwnProperty('outputHash')) {
      properties._outputHash = 1;
      if (!$util.isString(message.outputHash))
        return 'outputHash: string expected';
    }
    if (message.pipeline != null && message.hasOwnProperty('pipeline')) {
      properties._pipeline = 1;
      if (!$util.isString(message.pipeline)) return 'pipeline: string expected';
    }
    if (message.astKey != null && message.hasOwnProperty('astKey')) {
      properties._astKey = 1;
      if (!$util.isString(message.astKey)) return 'astKey: string expected';
    }
    if (
      message.astGenerator != null &&
      message.hasOwnProperty('astGenerator')
    ) {
      properties._astGenerator = 1;
      {
        let error = $root.ASTGenerator.verify(message.astGenerator);
        if (error) return 'astGenerator.' + error;
      }
    }
    if (message.symbols != null && message.hasOwnProperty('symbols')) {
      if (!$util.isObject(message.symbols)) return 'symbols: object expected';
      let key = Object.keys(message.symbols);
      for (let i = 0; i < key.length; ++i) {
        let error = $root.AssetSymbol.verify(message.symbols[key[i]]);
        if (error) return 'symbols.' + error;
      }
    }
    if (message.sideEffects != null && message.hasOwnProperty('sideEffects'))
      if (typeof message.sideEffects !== 'boolean')
        return 'sideEffects: boolean expected';
    if (message.uniqueKey != null && message.hasOwnProperty('uniqueKey')) {
      properties._uniqueKey = 1;
      if (!$util.isString(message.uniqueKey))
        return 'uniqueKey: string expected';
    }
    if (message.configPath != null && message.hasOwnProperty('configPath')) {
      properties._configPath = 1;
      if (!$util.isString(message.configPath))
        return 'configPath: string expected';
    }
    if (message.plugin != null && message.hasOwnProperty('plugin')) {
      properties._plugin = 1;
      if (!$util.isString(message.plugin)) return 'plugin: string expected';
    }
    if (
      message.configKeyPath != null &&
      message.hasOwnProperty('configKeyPath')
    ) {
      properties._configKeyPath = 1;
      if (!$util.isString(message.configKeyPath))
        return 'configKeyPath: string expected';
    }
    if (message.isLargeBlob != null && message.hasOwnProperty('isLargeBlob')) {
      properties._isLargeBlob = 1;
      if (typeof message.isLargeBlob !== 'boolean')
        return 'isLargeBlob: boolean expected';
    }
    return null;
  };

  /**
   * Creates an Asset message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof Asset
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {Asset} Asset
   */
  Asset.fromObject = function fromObject(object) {
    if (object instanceof $root.Asset) return object;
    let message = new $root.Asset();
    if (object.id != null) message.id = String(object.id);
    if (object.committed != null) message.committed = Boolean(object.committed);
    if (object.filePath != null) message.filePath = String(object.filePath);
    if (object.query != null) message.query = String(object.query);
    if (object.type != null) message.type = String(object.type);
    if (object.dependencies) {
      if (typeof object.dependencies !== 'object')
        throw TypeError('.Asset.dependencies: object expected');
      message.dependencies = {};
      for (
        let keys = Object.keys(object.dependencies), i = 0;
        i < keys.length;
        ++i
      ) {
        if (typeof object.dependencies[keys[i]] !== 'object')
          throw TypeError('.Asset.dependencies: object expected');
        message.dependencies[keys[i]] = $root.Dependency.fromObject(
          object.dependencies[keys[i]],
        );
      }
    }
    switch (object.bundleBehavior) {
      default:
        if (typeof object.bundleBehavior === 'number') {
          message.bundleBehavior = object.bundleBehavior;
          break;
        }
        break;
      case 'BUNDLE_BEHAVIOR_INLINE':
      case 0:
        message.bundleBehavior = 0;
        break;
      case 'BUNDLE_BEHAVIOR_ISOLATED':
      case 1:
        message.bundleBehavior = 1;
        break;
    }
    if (object.isBundleSplittable != null)
      message.isBundleSplittable = Boolean(object.isBundleSplittable);
    if (object.isSource != null) message.isSource = Boolean(object.isSource);
    if (object.env != null) message.env = String(object.env);
    if (object.meta != null) message.meta = String(object.meta);
    if (object.stats != null) message.stats = String(object.stats);
    if (object.contentKey != null)
      message.contentKey = String(object.contentKey);
    if (object.mapKey != null) message.mapKey = String(object.mapKey);
    if (object.outputHash != null)
      message.outputHash = String(object.outputHash);
    if (object.pipeline != null) message.pipeline = String(object.pipeline);
    if (object.astKey != null) message.astKey = String(object.astKey);
    if (object.astGenerator != null) {
      if (typeof object.astGenerator !== 'object')
        throw TypeError('.Asset.astGenerator: object expected');
      message.astGenerator = $root.ASTGenerator.fromObject(object.astGenerator);
    }
    if (object.symbols) {
      if (typeof object.symbols !== 'object')
        throw TypeError('.Asset.symbols: object expected');
      message.symbols = {};
      for (
        let keys = Object.keys(object.symbols), i = 0;
        i < keys.length;
        ++i
      ) {
        if (typeof object.symbols[keys[i]] !== 'object')
          throw TypeError('.Asset.symbols: object expected');
        message.symbols[keys[i]] = $root.AssetSymbol.fromObject(
          object.symbols[keys[i]],
        );
      }
    }
    if (object.sideEffects != null)
      message.sideEffects = Boolean(object.sideEffects);
    if (object.uniqueKey != null) message.uniqueKey = String(object.uniqueKey);
    if (object.configPath != null)
      message.configPath = String(object.configPath);
    if (object.plugin != null) message.plugin = String(object.plugin);
    if (object.configKeyPath != null)
      message.configKeyPath = String(object.configKeyPath);
    if (object.isLargeBlob != null)
      message.isLargeBlob = Boolean(object.isLargeBlob);
    return message;
  };

  /**
   * Creates a plain object from an Asset message. Also converts values to other types if specified.
   * @function toObject
   * @memberof Asset
   * @static
   * @param {Asset} message Asset
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  Asset.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.objects || options.defaults) {
      object.dependencies = {};
      object.symbols = {};
    }
    if (options.defaults) {
      object.id = '';
      object.committed = false;
      object.filePath = '';
      object.type = '';
      object.bundleBehavior =
        options.enums === String ? 'BUNDLE_BEHAVIOR_INLINE' : 0;
      object.isBundleSplittable = false;
      object.isSource = false;
      object.env = '';
      object.meta = '';
      object.stats = '';
      object.sideEffects = false;
    }
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.committed != null && message.hasOwnProperty('committed'))
      object.committed = message.committed;
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      object.filePath = message.filePath;
    if (message.query != null && message.hasOwnProperty('query')) {
      object.query = message.query;
      if (options.oneofs) object._query = 'query';
    }
    if (message.type != null && message.hasOwnProperty('type'))
      object.type = message.type;
    let keys2;
    if (
      message.dependencies &&
      (keys2 = Object.keys(message.dependencies)).length
    ) {
      object.dependencies = {};
      for (let j = 0; j < keys2.length; ++j)
        object.dependencies[keys2[j]] = $root.Dependency.toObject(
          message.dependencies[keys2[j]],
          options,
        );
    }
    if (
      message.bundleBehavior != null &&
      message.hasOwnProperty('bundleBehavior')
    )
      object.bundleBehavior =
        options.enums === String
          ? $root.BundleBehavior[message.bundleBehavior] === undefined
            ? message.bundleBehavior
            : $root.BundleBehavior[message.bundleBehavior]
          : message.bundleBehavior;
    if (
      message.isBundleSplittable != null &&
      message.hasOwnProperty('isBundleSplittable')
    )
      object.isBundleSplittable = message.isBundleSplittable;
    if (message.isSource != null && message.hasOwnProperty('isSource'))
      object.isSource = message.isSource;
    if (message.env != null && message.hasOwnProperty('env'))
      object.env = message.env;
    if (message.meta != null && message.hasOwnProperty('meta'))
      object.meta = message.meta;
    if (message.stats != null && message.hasOwnProperty('stats'))
      object.stats = message.stats;
    if (message.contentKey != null && message.hasOwnProperty('contentKey')) {
      object.contentKey = message.contentKey;
      if (options.oneofs) object._contentKey = 'contentKey';
    }
    if (message.mapKey != null && message.hasOwnProperty('mapKey')) {
      object.mapKey = message.mapKey;
      if (options.oneofs) object._mapKey = 'mapKey';
    }
    if (message.outputHash != null && message.hasOwnProperty('outputHash')) {
      object.outputHash = message.outputHash;
      if (options.oneofs) object._outputHash = 'outputHash';
    }
    if (message.pipeline != null && message.hasOwnProperty('pipeline')) {
      object.pipeline = message.pipeline;
      if (options.oneofs) object._pipeline = 'pipeline';
    }
    if (message.astKey != null && message.hasOwnProperty('astKey')) {
      object.astKey = message.astKey;
      if (options.oneofs) object._astKey = 'astKey';
    }
    if (
      message.astGenerator != null &&
      message.hasOwnProperty('astGenerator')
    ) {
      object.astGenerator = $root.ASTGenerator.toObject(
        message.astGenerator,
        options,
      );
      if (options.oneofs) object._astGenerator = 'astGenerator';
    }
    if (message.symbols && (keys2 = Object.keys(message.symbols)).length) {
      object.symbols = {};
      for (let j = 0; j < keys2.length; ++j)
        object.symbols[keys2[j]] = $root.AssetSymbol.toObject(
          message.symbols[keys2[j]],
          options,
        );
    }
    if (message.sideEffects != null && message.hasOwnProperty('sideEffects'))
      object.sideEffects = message.sideEffects;
    if (message.uniqueKey != null && message.hasOwnProperty('uniqueKey')) {
      object.uniqueKey = message.uniqueKey;
      if (options.oneofs) object._uniqueKey = 'uniqueKey';
    }
    if (message.configPath != null && message.hasOwnProperty('configPath')) {
      object.configPath = message.configPath;
      if (options.oneofs) object._configPath = 'configPath';
    }
    if (message.plugin != null && message.hasOwnProperty('plugin')) {
      object.plugin = message.plugin;
      if (options.oneofs) object._plugin = 'plugin';
    }
    if (
      message.configKeyPath != null &&
      message.hasOwnProperty('configKeyPath')
    ) {
      object.configKeyPath = message.configKeyPath;
      if (options.oneofs) object._configKeyPath = 'configKeyPath';
    }
    if (message.isLargeBlob != null && message.hasOwnProperty('isLargeBlob')) {
      object.isLargeBlob = message.isLargeBlob;
      if (options.oneofs) object._isLargeBlob = 'isLargeBlob';
    }
    return object;
  };

  /**
   * Converts this Asset to JSON.
   * @function toJSON
   * @memberof Asset
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  Asset.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for Asset
   * @function getTypeUrl
   * @memberof Asset
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  Asset.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/Asset';
  };

  return Asset;
})());

export const AssetGraphNode = ($root.AssetGraphNode = (() => {
  /**
   * Properties of an AssetGraphNode.
   * @exports IAssetGraphNode
   * @interface IAssetGraphNode
   * @property {IAssetGraphRootNode|null} [root] AssetGraphNode root
   * @property {IAssetGraphEntryFileNode|null} [entryFile] AssetGraphNode entryFile
   * @property {IAssetGraphEntrySpecifierNode|null} [entrySpecifier] AssetGraphNode entrySpecifier
   * @property {IAssetGraphDependencyNode|null} [dependency] AssetGraphNode dependency
   */

  /**
   * Constructs a new AssetGraphNode.
   * @exports AssetGraphNode
   * @classdesc Represents an AssetGraphNode.
   * @implements IAssetGraphNode
   * @constructor
   * @param {IAssetGraphNode=} [properties] Properties to set
   */
  function AssetGraphNode(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetGraphNode root.
   * @member {IAssetGraphRootNode|null|undefined} root
   * @memberof AssetGraphNode
   * @instance
   */
  AssetGraphNode.prototype.root = null;

  /**
   * AssetGraphNode entryFile.
   * @member {IAssetGraphEntryFileNode|null|undefined} entryFile
   * @memberof AssetGraphNode
   * @instance
   */
  AssetGraphNode.prototype.entryFile = null;

  /**
   * AssetGraphNode entrySpecifier.
   * @member {IAssetGraphEntrySpecifierNode|null|undefined} entrySpecifier
   * @memberof AssetGraphNode
   * @instance
   */
  AssetGraphNode.prototype.entrySpecifier = null;

  /**
   * AssetGraphNode dependency.
   * @member {IAssetGraphDependencyNode|null|undefined} dependency
   * @memberof AssetGraphNode
   * @instance
   */
  AssetGraphNode.prototype.dependency = null;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  /**
   * AssetGraphNode value.
   * @member {"root"|"entryFile"|"entrySpecifier"|"dependency"|undefined} value
   * @memberof AssetGraphNode
   * @instance
   */
  Object.defineProperty(AssetGraphNode.prototype, 'value', {
    get: $util.oneOfGetter(
      ($oneOfFields = ['root', 'entryFile', 'entrySpecifier', 'dependency']),
    ),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new AssetGraphNode instance using the specified properties.
   * @function create
   * @memberof AssetGraphNode
   * @static
   * @param {IAssetGraphNode=} [properties] Properties to set
   * @returns {AssetGraphNode} AssetGraphNode instance
   */
  AssetGraphNode.create = function create(properties) {
    return new AssetGraphNode(properties);
  };

  /**
   * Encodes the specified AssetGraphNode message. Does not implicitly {@link AssetGraphNode.verify|verify} messages.
   * @function encode
   * @memberof AssetGraphNode
   * @static
   * @param {IAssetGraphNode} message AssetGraphNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphNode.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.root != null && Object.hasOwnProperty.call(message, 'root'))
      $root.AssetGraphRootNode.encode(
        message.root,
        writer.uint32(/* id 1, wireType 2 =*/ 10).fork(),
      ).ldelim();
    if (
      message.entryFile != null &&
      Object.hasOwnProperty.call(message, 'entryFile')
    )
      $root.AssetGraphEntryFileNode.encode(
        message.entryFile,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    if (
      message.entrySpecifier != null &&
      Object.hasOwnProperty.call(message, 'entrySpecifier')
    )
      $root.AssetGraphEntrySpecifierNode.encode(
        message.entrySpecifier,
        writer.uint32(/* id 3, wireType 2 =*/ 26).fork(),
      ).ldelim();
    if (
      message.dependency != null &&
      Object.hasOwnProperty.call(message, 'dependency')
    )
      $root.AssetGraphDependencyNode.encode(
        message.dependency,
        writer.uint32(/* id 4, wireType 2 =*/ 34).fork(),
      ).ldelim();
    return writer;
  };

  /**
   * Encodes the specified AssetGraphNode message, length delimited. Does not implicitly {@link AssetGraphNode.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetGraphNode
   * @static
   * @param {IAssetGraphNode} message AssetGraphNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetGraphNode.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetGraphNode message from the specified reader or buffer.
   * @function decode
   * @memberof AssetGraphNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetGraphNode} AssetGraphNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphNode.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetGraphNode();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.root = $root.AssetGraphRootNode.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        case 2: {
          message.entryFile = $root.AssetGraphEntryFileNode.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        case 3: {
          message.entrySpecifier = $root.AssetGraphEntrySpecifierNode.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        case 4: {
          message.dependency = $root.AssetGraphDependencyNode.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetGraphNode message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetGraphNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetGraphNode} AssetGraphNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetGraphNode.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetGraphNode message.
   * @function verify
   * @memberof AssetGraphNode
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetGraphNode.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.root != null && message.hasOwnProperty('root')) {
      properties.value = 1;
      {
        let error = $root.AssetGraphRootNode.verify(message.root);
        if (error) return 'root.' + error;
      }
    }
    if (message.entryFile != null && message.hasOwnProperty('entryFile')) {
      if (properties.value === 1) return 'value: multiple values';
      properties.value = 1;
      {
        let error = $root.AssetGraphEntryFileNode.verify(message.entryFile);
        if (error) return 'entryFile.' + error;
      }
    }
    if (
      message.entrySpecifier != null &&
      message.hasOwnProperty('entrySpecifier')
    ) {
      if (properties.value === 1) return 'value: multiple values';
      properties.value = 1;
      {
        let error = $root.AssetGraphEntrySpecifierNode.verify(
          message.entrySpecifier,
        );
        if (error) return 'entrySpecifier.' + error;
      }
    }
    if (message.dependency != null && message.hasOwnProperty('dependency')) {
      if (properties.value === 1) return 'value: multiple values';
      properties.value = 1;
      {
        let error = $root.AssetGraphDependencyNode.verify(message.dependency);
        if (error) return 'dependency.' + error;
      }
    }
    return null;
  };

  /**
   * Creates an AssetGraphNode message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetGraphNode
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetGraphNode} AssetGraphNode
   */
  AssetGraphNode.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetGraphNode) return object;
    let message = new $root.AssetGraphNode();
    if (object.root != null) {
      if (typeof object.root !== 'object')
        throw TypeError('.AssetGraphNode.root: object expected');
      message.root = $root.AssetGraphRootNode.fromObject(object.root);
    }
    if (object.entryFile != null) {
      if (typeof object.entryFile !== 'object')
        throw TypeError('.AssetGraphNode.entryFile: object expected');
      message.entryFile = $root.AssetGraphEntryFileNode.fromObject(
        object.entryFile,
      );
    }
    if (object.entrySpecifier != null) {
      if (typeof object.entrySpecifier !== 'object')
        throw TypeError('.AssetGraphNode.entrySpecifier: object expected');
      message.entrySpecifier = $root.AssetGraphEntrySpecifierNode.fromObject(
        object.entrySpecifier,
      );
    }
    if (object.dependency != null) {
      if (typeof object.dependency !== 'object')
        throw TypeError('.AssetGraphNode.dependency: object expected');
      message.dependency = $root.AssetGraphDependencyNode.fromObject(
        object.dependency,
      );
    }
    return message;
  };

  /**
   * Creates a plain object from an AssetGraphNode message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetGraphNode
   * @static
   * @param {AssetGraphNode} message AssetGraphNode
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetGraphNode.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (message.root != null && message.hasOwnProperty('root')) {
      object.root = $root.AssetGraphRootNode.toObject(message.root, options);
      if (options.oneofs) object.value = 'root';
    }
    if (message.entryFile != null && message.hasOwnProperty('entryFile')) {
      object.entryFile = $root.AssetGraphEntryFileNode.toObject(
        message.entryFile,
        options,
      );
      if (options.oneofs) object.value = 'entryFile';
    }
    if (
      message.entrySpecifier != null &&
      message.hasOwnProperty('entrySpecifier')
    ) {
      object.entrySpecifier = $root.AssetGraphEntrySpecifierNode.toObject(
        message.entrySpecifier,
        options,
      );
      if (options.oneofs) object.value = 'entrySpecifier';
    }
    if (message.dependency != null && message.hasOwnProperty('dependency')) {
      object.dependency = $root.AssetGraphDependencyNode.toObject(
        message.dependency,
        options,
      );
      if (options.oneofs) object.value = 'dependency';
    }
    return object;
  };

  /**
   * Converts this AssetGraphNode to JSON.
   * @function toJSON
   * @memberof AssetGraphNode
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetGraphNode.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetGraphNode
   * @function getTypeUrl
   * @memberof AssetGraphNode
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetGraphNode.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetGraphNode';
  };

  return AssetGraphNode;
})());

export const Dependency = ($root.Dependency = (() => {
  /**
   * Properties of a Dependency.
   * @exports IDependency
   * @interface IDependency
   * @property {string|null} [id] Dependency id
   * @property {string|null} [specifier] Dependency specifier
   * @property {SpecifierType|null} [specifierType] Dependency specifierType
   * @property {DependencyPriority|null} [priority] Dependency priority
   * @property {BundleBehavior|null} [bundleBehavior] Dependency bundleBehavior
   * @property {boolean|null} [needsStableName] Dependency needsStableName
   * @property {boolean|null} [isOptional] Dependency isOptional
   * @property {boolean|null} [isEntry] Dependency isEntry
   * @property {ISourceLocation|null} [loc] Dependency loc
   * @property {string|null} [environmentId] Dependency environmentId
   * @property {Array.<string>|null} [packageConditions] Dependency packageConditions
   * @property {string|null} [meta] Dependency meta
   * @property {ITarget|null} [target] Dependency target
   * @property {string|null} [sourceAssetId] Dependency sourceAssetId
   * @property {string|null} [sourcePath] Dependency sourcePath
   * @property {string|null} [sourceAssetType] Dependency sourceAssetType
   * @property {string|null} [resolveFrom] Dependency resolveFrom
   * @property {string|null} [range] Dependency range
   * @property {string|null} [pipeline] Dependency pipeline
   * @property {Object.<string,IDependencySymbol>|null} [symbols] Dependency symbols
   */

  /**
   * Constructs a new Dependency.
   * @exports Dependency
   * @classdesc Represents a Dependency.
   * @implements IDependency
   * @constructor
   * @param {IDependency=} [properties] Properties to set
   */
  function Dependency(properties) {
    this.packageConditions = [];
    this.symbols = {};
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * Dependency id.
   * @member {string} id
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.id = '';

  /**
   * Dependency specifier.
   * @member {string} specifier
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.specifier = '';

  /**
   * Dependency specifierType.
   * @member {SpecifierType} specifierType
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.specifierType = 0;

  /**
   * Dependency priority.
   * @member {DependencyPriority} priority
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.priority = 0;

  /**
   * Dependency bundleBehavior.
   * @member {BundleBehavior} bundleBehavior
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.bundleBehavior = 0;

  /**
   * Dependency needsStableName.
   * @member {boolean} needsStableName
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.needsStableName = false;

  /**
   * Dependency isOptional.
   * @member {boolean} isOptional
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.isOptional = false;

  /**
   * Dependency isEntry.
   * @member {boolean} isEntry
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.isEntry = false;

  /**
   * Dependency loc.
   * @member {ISourceLocation|null|undefined} loc
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.loc = null;

  /**
   * Dependency environmentId.
   * @member {string} environmentId
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.environmentId = '';

  /**
   * Dependency packageConditions.
   * @member {Array.<string>} packageConditions
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.packageConditions = $util.emptyArray;

  /**
   * Dependency meta.
   * @member {string} meta
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.meta = '';

  /**
   * Dependency target.
   * @member {ITarget|null|undefined} target
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.target = null;

  /**
   * Dependency sourceAssetId.
   * @member {string} sourceAssetId
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.sourceAssetId = '';

  /**
   * Dependency sourcePath.
   * @member {string} sourcePath
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.sourcePath = '';

  /**
   * Dependency sourceAssetType.
   * @member {string} sourceAssetType
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.sourceAssetType = '';

  /**
   * Dependency resolveFrom.
   * @member {string} resolveFrom
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.resolveFrom = '';

  /**
   * Dependency range.
   * @member {string} range
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.range = '';

  /**
   * Dependency pipeline.
   * @member {string} pipeline
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.pipeline = '';

  /**
   * Dependency symbols.
   * @member {Object.<string,IDependencySymbol>} symbols
   * @memberof Dependency
   * @instance
   */
  Dependency.prototype.symbols = $util.emptyObject;

  // OneOf field names bound to virtual getters and setters
  let $oneOfFields;

  // Virtual OneOf for proto3 optional field
  Object.defineProperty(Dependency.prototype, '_target', {
    get: $util.oneOfGetter(($oneOfFields = ['target'])),
    set: $util.oneOfSetter($oneOfFields),
  });

  /**
   * Creates a new Dependency instance using the specified properties.
   * @function create
   * @memberof Dependency
   * @static
   * @param {IDependency=} [properties] Properties to set
   * @returns {Dependency} Dependency instance
   */
  Dependency.create = function create(properties) {
    return new Dependency(properties);
  };

  /**
   * Encodes the specified Dependency message. Does not implicitly {@link Dependency.verify|verify} messages.
   * @function encode
   * @memberof Dependency
   * @static
   * @param {IDependency} message Dependency message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Dependency.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (
      message.specifier != null &&
      Object.hasOwnProperty.call(message, 'specifier')
    )
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.specifier);
    if (
      message.specifierType != null &&
      Object.hasOwnProperty.call(message, 'specifierType')
    )
      writer.uint32(/* id 3, wireType 0 =*/ 24).int32(message.specifierType);
    if (
      message.priority != null &&
      Object.hasOwnProperty.call(message, 'priority')
    )
      writer.uint32(/* id 4, wireType 0 =*/ 32).int32(message.priority);
    if (
      message.bundleBehavior != null &&
      Object.hasOwnProperty.call(message, 'bundleBehavior')
    )
      writer.uint32(/* id 5, wireType 0 =*/ 40).int32(message.bundleBehavior);
    if (
      message.needsStableName != null &&
      Object.hasOwnProperty.call(message, 'needsStableName')
    )
      writer.uint32(/* id 6, wireType 0 =*/ 48).bool(message.needsStableName);
    if (
      message.isOptional != null &&
      Object.hasOwnProperty.call(message, 'isOptional')
    )
      writer.uint32(/* id 7, wireType 0 =*/ 56).bool(message.isOptional);
    if (
      message.isEntry != null &&
      Object.hasOwnProperty.call(message, 'isEntry')
    )
      writer.uint32(/* id 8, wireType 0 =*/ 64).bool(message.isEntry);
    if (message.loc != null && Object.hasOwnProperty.call(message, 'loc'))
      $root.SourceLocation.encode(
        message.loc,
        writer.uint32(/* id 9, wireType 2 =*/ 74).fork(),
      ).ldelim();
    if (
      message.environmentId != null &&
      Object.hasOwnProperty.call(message, 'environmentId')
    )
      writer.uint32(/* id 10, wireType 2 =*/ 82).string(message.environmentId);
    if (message.packageConditions != null && message.packageConditions.length)
      for (let i = 0; i < message.packageConditions.length; ++i)
        writer
          .uint32(/* id 11, wireType 2 =*/ 90)
          .string(message.packageConditions[i]);
    if (message.meta != null && Object.hasOwnProperty.call(message, 'meta'))
      writer.uint32(/* id 12, wireType 2 =*/ 98).string(message.meta);
    if (message.target != null && Object.hasOwnProperty.call(message, 'target'))
      $root.Target.encode(
        message.target,
        writer.uint32(/* id 13, wireType 2 =*/ 106).fork(),
      ).ldelim();
    if (
      message.sourceAssetId != null &&
      Object.hasOwnProperty.call(message, 'sourceAssetId')
    )
      writer.uint32(/* id 14, wireType 2 =*/ 114).string(message.sourceAssetId);
    if (
      message.sourcePath != null &&
      Object.hasOwnProperty.call(message, 'sourcePath')
    )
      writer.uint32(/* id 15, wireType 2 =*/ 122).string(message.sourcePath);
    if (
      message.sourceAssetType != null &&
      Object.hasOwnProperty.call(message, 'sourceAssetType')
    )
      writer
        .uint32(/* id 16, wireType 2 =*/ 130)
        .string(message.sourceAssetType);
    if (
      message.resolveFrom != null &&
      Object.hasOwnProperty.call(message, 'resolveFrom')
    )
      writer.uint32(/* id 17, wireType 2 =*/ 138).string(message.resolveFrom);
    if (message.range != null && Object.hasOwnProperty.call(message, 'range'))
      writer.uint32(/* id 18, wireType 2 =*/ 146).string(message.range);
    if (
      message.pipeline != null &&
      Object.hasOwnProperty.call(message, 'pipeline')
    )
      writer.uint32(/* id 19, wireType 2 =*/ 154).string(message.pipeline);
    if (
      message.symbols != null &&
      Object.hasOwnProperty.call(message, 'symbols')
    )
      for (
        let keys = Object.keys(message.symbols), i = 0;
        i < keys.length;
        ++i
      ) {
        writer
          .uint32(/* id 20, wireType 2 =*/ 162)
          .fork()
          .uint32(/* id 1, wireType 2 =*/ 10)
          .string(keys[i]);
        $root.DependencySymbol.encode(
          message.symbols[keys[i]],
          writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
        )
          .ldelim()
          .ldelim();
      }
    return writer;
  };

  /**
   * Encodes the specified Dependency message, length delimited. Does not implicitly {@link Dependency.verify|verify} messages.
   * @function encodeDelimited
   * @memberof Dependency
   * @static
   * @param {IDependency} message Dependency message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  Dependency.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes a Dependency message from the specified reader or buffer.
   * @function decode
   * @memberof Dependency
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {Dependency} Dependency
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Dependency.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.Dependency(),
      key,
      value;
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.specifier = reader.string();
          break;
        }
        case 3: {
          message.specifierType = reader.int32();
          break;
        }
        case 4: {
          message.priority = reader.int32();
          break;
        }
        case 5: {
          message.bundleBehavior = reader.int32();
          break;
        }
        case 6: {
          message.needsStableName = reader.bool();
          break;
        }
        case 7: {
          message.isOptional = reader.bool();
          break;
        }
        case 8: {
          message.isEntry = reader.bool();
          break;
        }
        case 9: {
          message.loc = $root.SourceLocation.decode(reader, reader.uint32());
          break;
        }
        case 10: {
          message.environmentId = reader.string();
          break;
        }
        case 11: {
          if (!(message.packageConditions && message.packageConditions.length))
            message.packageConditions = [];
          message.packageConditions.push(reader.string());
          break;
        }
        case 12: {
          message.meta = reader.string();
          break;
        }
        case 13: {
          message.target = $root.Target.decode(reader, reader.uint32());
          break;
        }
        case 14: {
          message.sourceAssetId = reader.string();
          break;
        }
        case 15: {
          message.sourcePath = reader.string();
          break;
        }
        case 16: {
          message.sourceAssetType = reader.string();
          break;
        }
        case 17: {
          message.resolveFrom = reader.string();
          break;
        }
        case 18: {
          message.range = reader.string();
          break;
        }
        case 19: {
          message.pipeline = reader.string();
          break;
        }
        case 20: {
          if (message.symbols === $util.emptyObject) message.symbols = {};
          let end2 = reader.uint32() + reader.pos;
          key = '';
          value = null;
          while (reader.pos < end2) {
            let tag2 = reader.uint32();
            switch (tag2 >>> 3) {
              case 1:
                key = reader.string();
                break;
              case 2:
                value = $root.DependencySymbol.decode(reader, reader.uint32());
                break;
              default:
                reader.skipType(tag2 & 7);
                break;
            }
          }
          message.symbols[key] = value;
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes a Dependency message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof Dependency
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {Dependency} Dependency
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  Dependency.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies a Dependency message.
   * @function verify
   * @memberof Dependency
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  Dependency.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    let properties = {};
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.specifier != null && message.hasOwnProperty('specifier'))
      if (!$util.isString(message.specifier))
        return 'specifier: string expected';
    if (
      message.specifierType != null &&
      message.hasOwnProperty('specifierType')
    )
      switch (message.specifierType) {
        default:
          return 'specifierType: enum value expected';
        case 0:
        case 1:
        case 2:
        case 3:
          break;
      }
    if (message.priority != null && message.hasOwnProperty('priority'))
      switch (message.priority) {
        default:
          return 'priority: enum value expected';
        case 0:
        case 1:
        case 2:
        case 3:
          break;
      }
    if (
      message.bundleBehavior != null &&
      message.hasOwnProperty('bundleBehavior')
    )
      switch (message.bundleBehavior) {
        default:
          return 'bundleBehavior: enum value expected';
        case 0:
        case 1:
          break;
      }
    if (
      message.needsStableName != null &&
      message.hasOwnProperty('needsStableName')
    )
      if (typeof message.needsStableName !== 'boolean')
        return 'needsStableName: boolean expected';
    if (message.isOptional != null && message.hasOwnProperty('isOptional'))
      if (typeof message.isOptional !== 'boolean')
        return 'isOptional: boolean expected';
    if (message.isEntry != null && message.hasOwnProperty('isEntry'))
      if (typeof message.isEntry !== 'boolean')
        return 'isEntry: boolean expected';
    if (message.loc != null && message.hasOwnProperty('loc')) {
      let error = $root.SourceLocation.verify(message.loc);
      if (error) return 'loc.' + error;
    }
    if (
      message.environmentId != null &&
      message.hasOwnProperty('environmentId')
    )
      if (!$util.isString(message.environmentId))
        return 'environmentId: string expected';
    if (
      message.packageConditions != null &&
      message.hasOwnProperty('packageConditions')
    ) {
      if (!Array.isArray(message.packageConditions))
        return 'packageConditions: array expected';
      for (let i = 0; i < message.packageConditions.length; ++i)
        if (!$util.isString(message.packageConditions[i]))
          return 'packageConditions: string[] expected';
    }
    if (message.meta != null && message.hasOwnProperty('meta'))
      if (!$util.isString(message.meta)) return 'meta: string expected';
    if (message.target != null && message.hasOwnProperty('target')) {
      properties._target = 1;
      {
        let error = $root.Target.verify(message.target);
        if (error) return 'target.' + error;
      }
    }
    if (
      message.sourceAssetId != null &&
      message.hasOwnProperty('sourceAssetId')
    )
      if (!$util.isString(message.sourceAssetId))
        return 'sourceAssetId: string expected';
    if (message.sourcePath != null && message.hasOwnProperty('sourcePath'))
      if (!$util.isString(message.sourcePath))
        return 'sourcePath: string expected';
    if (
      message.sourceAssetType != null &&
      message.hasOwnProperty('sourceAssetType')
    )
      if (!$util.isString(message.sourceAssetType))
        return 'sourceAssetType: string expected';
    if (message.resolveFrom != null && message.hasOwnProperty('resolveFrom'))
      if (!$util.isString(message.resolveFrom))
        return 'resolveFrom: string expected';
    if (message.range != null && message.hasOwnProperty('range'))
      if (!$util.isString(message.range)) return 'range: string expected';
    if (message.pipeline != null && message.hasOwnProperty('pipeline'))
      if (!$util.isString(message.pipeline)) return 'pipeline: string expected';
    if (message.symbols != null && message.hasOwnProperty('symbols')) {
      if (!$util.isObject(message.symbols)) return 'symbols: object expected';
      let key = Object.keys(message.symbols);
      for (let i = 0; i < key.length; ++i) {
        let error = $root.DependencySymbol.verify(message.symbols[key[i]]);
        if (error) return 'symbols.' + error;
      }
    }
    return null;
  };

  /**
   * Creates a Dependency message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof Dependency
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {Dependency} Dependency
   */
  Dependency.fromObject = function fromObject(object) {
    if (object instanceof $root.Dependency) return object;
    let message = new $root.Dependency();
    if (object.id != null) message.id = String(object.id);
    if (object.specifier != null) message.specifier = String(object.specifier);
    switch (object.specifierType) {
      default:
        if (typeof object.specifierType === 'number') {
          message.specifierType = object.specifierType;
          break;
        }
        break;
      case 'SPECIFIER_TYPE_COMMONJS':
      case 0:
        message.specifierType = 0;
        break;
      case 'SPECIFIER_TYPE_ESM':
      case 1:
        message.specifierType = 1;
        break;
      case 'SPECIFIER_TYPE_URL':
      case 2:
        message.specifierType = 2;
        break;
      case 'SPECIFIER_TYPE_CUSTOM':
      case 3:
        message.specifierType = 3;
        break;
    }
    switch (object.priority) {
      default:
        if (typeof object.priority === 'number') {
          message.priority = object.priority;
          break;
        }
        break;
      case 'DEPENDENCY_PRIORITY_SYNC':
      case 0:
        message.priority = 0;
        break;
      case 'DEPENDENCY_PRIORITY_PARALLEL':
      case 1:
        message.priority = 1;
        break;
      case 'DEPENDENCY_PRIORITY_LAZY':
      case 2:
        message.priority = 2;
        break;
      case 'DEPENDENCY_PRIORITY_CONDITIONAL':
      case 3:
        message.priority = 3;
        break;
    }
    switch (object.bundleBehavior) {
      default:
        if (typeof object.bundleBehavior === 'number') {
          message.bundleBehavior = object.bundleBehavior;
          break;
        }
        break;
      case 'BUNDLE_BEHAVIOR_INLINE':
      case 0:
        message.bundleBehavior = 0;
        break;
      case 'BUNDLE_BEHAVIOR_ISOLATED':
      case 1:
        message.bundleBehavior = 1;
        break;
    }
    if (object.needsStableName != null)
      message.needsStableName = Boolean(object.needsStableName);
    if (object.isOptional != null)
      message.isOptional = Boolean(object.isOptional);
    if (object.isEntry != null) message.isEntry = Boolean(object.isEntry);
    if (object.loc != null) {
      if (typeof object.loc !== 'object')
        throw TypeError('.Dependency.loc: object expected');
      message.loc = $root.SourceLocation.fromObject(object.loc);
    }
    if (object.environmentId != null)
      message.environmentId = String(object.environmentId);
    if (object.packageConditions) {
      if (!Array.isArray(object.packageConditions))
        throw TypeError('.Dependency.packageConditions: array expected');
      message.packageConditions = [];
      for (let i = 0; i < object.packageConditions.length; ++i)
        message.packageConditions[i] = String(object.packageConditions[i]);
    }
    if (object.meta != null) message.meta = String(object.meta);
    if (object.target != null) {
      if (typeof object.target !== 'object')
        throw TypeError('.Dependency.target: object expected');
      message.target = $root.Target.fromObject(object.target);
    }
    if (object.sourceAssetId != null)
      message.sourceAssetId = String(object.sourceAssetId);
    if (object.sourcePath != null)
      message.sourcePath = String(object.sourcePath);
    if (object.sourceAssetType != null)
      message.sourceAssetType = String(object.sourceAssetType);
    if (object.resolveFrom != null)
      message.resolveFrom = String(object.resolveFrom);
    if (object.range != null) message.range = String(object.range);
    if (object.pipeline != null) message.pipeline = String(object.pipeline);
    if (object.symbols) {
      if (typeof object.symbols !== 'object')
        throw TypeError('.Dependency.symbols: object expected');
      message.symbols = {};
      for (
        let keys = Object.keys(object.symbols), i = 0;
        i < keys.length;
        ++i
      ) {
        if (typeof object.symbols[keys[i]] !== 'object')
          throw TypeError('.Dependency.symbols: object expected');
        message.symbols[keys[i]] = $root.DependencySymbol.fromObject(
          object.symbols[keys[i]],
        );
      }
    }
    return message;
  };

  /**
   * Creates a plain object from a Dependency message. Also converts values to other types if specified.
   * @function toObject
   * @memberof Dependency
   * @static
   * @param {Dependency} message Dependency
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  Dependency.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.arrays || options.defaults) object.packageConditions = [];
    if (options.objects || options.defaults) object.symbols = {};
    if (options.defaults) {
      object.id = '';
      object.specifier = '';
      object.specifierType =
        options.enums === String ? 'SPECIFIER_TYPE_COMMONJS' : 0;
      object.priority =
        options.enums === String ? 'DEPENDENCY_PRIORITY_SYNC' : 0;
      object.bundleBehavior =
        options.enums === String ? 'BUNDLE_BEHAVIOR_INLINE' : 0;
      object.needsStableName = false;
      object.isOptional = false;
      object.isEntry = false;
      object.loc = null;
      object.environmentId = '';
      object.meta = '';
      object.sourceAssetId = '';
      object.sourcePath = '';
      object.sourceAssetType = '';
      object.resolveFrom = '';
      object.range = '';
      object.pipeline = '';
    }
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.specifier != null && message.hasOwnProperty('specifier'))
      object.specifier = message.specifier;
    if (
      message.specifierType != null &&
      message.hasOwnProperty('specifierType')
    )
      object.specifierType =
        options.enums === String
          ? $root.SpecifierType[message.specifierType] === undefined
            ? message.specifierType
            : $root.SpecifierType[message.specifierType]
          : message.specifierType;
    if (message.priority != null && message.hasOwnProperty('priority'))
      object.priority =
        options.enums === String
          ? $root.DependencyPriority[message.priority] === undefined
            ? message.priority
            : $root.DependencyPriority[message.priority]
          : message.priority;
    if (
      message.bundleBehavior != null &&
      message.hasOwnProperty('bundleBehavior')
    )
      object.bundleBehavior =
        options.enums === String
          ? $root.BundleBehavior[message.bundleBehavior] === undefined
            ? message.bundleBehavior
            : $root.BundleBehavior[message.bundleBehavior]
          : message.bundleBehavior;
    if (
      message.needsStableName != null &&
      message.hasOwnProperty('needsStableName')
    )
      object.needsStableName = message.needsStableName;
    if (message.isOptional != null && message.hasOwnProperty('isOptional'))
      object.isOptional = message.isOptional;
    if (message.isEntry != null && message.hasOwnProperty('isEntry'))
      object.isEntry = message.isEntry;
    if (message.loc != null && message.hasOwnProperty('loc'))
      object.loc = $root.SourceLocation.toObject(message.loc, options);
    if (
      message.environmentId != null &&
      message.hasOwnProperty('environmentId')
    )
      object.environmentId = message.environmentId;
    if (message.packageConditions && message.packageConditions.length) {
      object.packageConditions = [];
      for (let j = 0; j < message.packageConditions.length; ++j)
        object.packageConditions[j] = message.packageConditions[j];
    }
    if (message.meta != null && message.hasOwnProperty('meta'))
      object.meta = message.meta;
    if (message.target != null && message.hasOwnProperty('target')) {
      object.target = $root.Target.toObject(message.target, options);
      if (options.oneofs) object._target = 'target';
    }
    if (
      message.sourceAssetId != null &&
      message.hasOwnProperty('sourceAssetId')
    )
      object.sourceAssetId = message.sourceAssetId;
    if (message.sourcePath != null && message.hasOwnProperty('sourcePath'))
      object.sourcePath = message.sourcePath;
    if (
      message.sourceAssetType != null &&
      message.hasOwnProperty('sourceAssetType')
    )
      object.sourceAssetType = message.sourceAssetType;
    if (message.resolveFrom != null && message.hasOwnProperty('resolveFrom'))
      object.resolveFrom = message.resolveFrom;
    if (message.range != null && message.hasOwnProperty('range'))
      object.range = message.range;
    if (message.pipeline != null && message.hasOwnProperty('pipeline'))
      object.pipeline = message.pipeline;
    let keys2;
    if (message.symbols && (keys2 = Object.keys(message.symbols)).length) {
      object.symbols = {};
      for (let j = 0; j < keys2.length; ++j)
        object.symbols[keys2[j]] = $root.DependencySymbol.toObject(
          message.symbols[keys2[j]],
          options,
        );
    }
    return object;
  };

  /**
   * Converts this Dependency to JSON.
   * @function toJSON
   * @memberof Dependency
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  Dependency.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for Dependency
   * @function getTypeUrl
   * @memberof Dependency
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  Dependency.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/Dependency';
  };

  return Dependency;
})());

export const AssetRequestInput = ($root.AssetRequestInput = (() => {
  /**
   * Properties of an AssetRequestInput.
   * @exports IAssetRequestInput
   * @interface IAssetRequestInput
   * @property {string|null} [name] AssetRequestInput name
   * @property {string|null} [filePath] AssetRequestInput filePath
   * @property {string|null} [env] AssetRequestInput env
   * @property {boolean|null} [isSource] AssetRequestInput isSource
   * @property {boolean|null} [canDefer] AssetRequestInput canDefer
   * @property {boolean|null} [sideEffects] AssetRequestInput sideEffects
   * @property {string|null} [code] AssetRequestInput code
   * @property {string|null} [pipeline] AssetRequestInput pipeline
   * @property {boolean|null} [isURL] AssetRequestInput isURL
   * @property {string|null} [query] AssetRequestInput query
   * @property {boolean|null} [isSingleChangeRebuild] AssetRequestInput isSingleChangeRebuild
   * @property {string|null} [optionsId] AssetRequestInput optionsId
   */

  /**
   * Constructs a new AssetRequestInput.
   * @exports AssetRequestInput
   * @classdesc Represents an AssetRequestInput.
   * @implements IAssetRequestInput
   * @constructor
   * @param {IAssetRequestInput=} [properties] Properties to set
   */
  function AssetRequestInput(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetRequestInput name.
   * @member {string} name
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.name = '';

  /**
   * AssetRequestInput filePath.
   * @member {string} filePath
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.filePath = '';

  /**
   * AssetRequestInput env.
   * @member {string} env
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.env = '';

  /**
   * AssetRequestInput isSource.
   * @member {boolean} isSource
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.isSource = false;

  /**
   * AssetRequestInput canDefer.
   * @member {boolean} canDefer
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.canDefer = false;

  /**
   * AssetRequestInput sideEffects.
   * @member {boolean} sideEffects
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.sideEffects = false;

  /**
   * AssetRequestInput code.
   * @member {string} code
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.code = '';

  /**
   * AssetRequestInput pipeline.
   * @member {string} pipeline
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.pipeline = '';

  /**
   * AssetRequestInput isURL.
   * @member {boolean} isURL
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.isURL = false;

  /**
   * AssetRequestInput query.
   * @member {string} query
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.query = '';

  /**
   * AssetRequestInput isSingleChangeRebuild.
   * @member {boolean} isSingleChangeRebuild
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.isSingleChangeRebuild = false;

  /**
   * AssetRequestInput optionsId.
   * @member {string} optionsId
   * @memberof AssetRequestInput
   * @instance
   */
  AssetRequestInput.prototype.optionsId = '';

  /**
   * Creates a new AssetRequestInput instance using the specified properties.
   * @function create
   * @memberof AssetRequestInput
   * @static
   * @param {IAssetRequestInput=} [properties] Properties to set
   * @returns {AssetRequestInput} AssetRequestInput instance
   */
  AssetRequestInput.create = function create(properties) {
    return new AssetRequestInput(properties);
  };

  /**
   * Encodes the specified AssetRequestInput message. Does not implicitly {@link AssetRequestInput.verify|verify} messages.
   * @function encode
   * @memberof AssetRequestInput
   * @static
   * @param {IAssetRequestInput} message AssetRequestInput message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetRequestInput.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.name != null && Object.hasOwnProperty.call(message, 'name'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.name);
    if (
      message.filePath != null &&
      Object.hasOwnProperty.call(message, 'filePath')
    )
      writer.uint32(/* id 2, wireType 2 =*/ 18).string(message.filePath);
    if (message.env != null && Object.hasOwnProperty.call(message, 'env'))
      writer.uint32(/* id 3, wireType 2 =*/ 26).string(message.env);
    if (
      message.isSource != null &&
      Object.hasOwnProperty.call(message, 'isSource')
    )
      writer.uint32(/* id 4, wireType 0 =*/ 32).bool(message.isSource);
    if (
      message.canDefer != null &&
      Object.hasOwnProperty.call(message, 'canDefer')
    )
      writer.uint32(/* id 5, wireType 0 =*/ 40).bool(message.canDefer);
    if (
      message.sideEffects != null &&
      Object.hasOwnProperty.call(message, 'sideEffects')
    )
      writer.uint32(/* id 6, wireType 0 =*/ 48).bool(message.sideEffects);
    if (message.code != null && Object.hasOwnProperty.call(message, 'code'))
      writer.uint32(/* id 7, wireType 2 =*/ 58).string(message.code);
    if (
      message.pipeline != null &&
      Object.hasOwnProperty.call(message, 'pipeline')
    )
      writer.uint32(/* id 8, wireType 2 =*/ 66).string(message.pipeline);
    if (message.isURL != null && Object.hasOwnProperty.call(message, 'isURL'))
      writer.uint32(/* id 10, wireType 0 =*/ 80).bool(message.isURL);
    if (message.query != null && Object.hasOwnProperty.call(message, 'query'))
      writer.uint32(/* id 11, wireType 2 =*/ 90).string(message.query);
    if (
      message.isSingleChangeRebuild != null &&
      Object.hasOwnProperty.call(message, 'isSingleChangeRebuild')
    )
      writer
        .uint32(/* id 12, wireType 0 =*/ 96)
        .bool(message.isSingleChangeRebuild);
    if (
      message.optionsId != null &&
      Object.hasOwnProperty.call(message, 'optionsId')
    )
      writer.uint32(/* id 13, wireType 2 =*/ 106).string(message.optionsId);
    return writer;
  };

  /**
   * Encodes the specified AssetRequestInput message, length delimited. Does not implicitly {@link AssetRequestInput.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetRequestInput
   * @static
   * @param {IAssetRequestInput} message AssetRequestInput message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetRequestInput.encodeDelimited = function encodeDelimited(
    message,
    writer,
  ) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetRequestInput message from the specified reader or buffer.
   * @function decode
   * @memberof AssetRequestInput
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetRequestInput} AssetRequestInput
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetRequestInput.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetRequestInput();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.name = reader.string();
          break;
        }
        case 2: {
          message.filePath = reader.string();
          break;
        }
        case 3: {
          message.env = reader.string();
          break;
        }
        case 4: {
          message.isSource = reader.bool();
          break;
        }
        case 5: {
          message.canDefer = reader.bool();
          break;
        }
        case 6: {
          message.sideEffects = reader.bool();
          break;
        }
        case 7: {
          message.code = reader.string();
          break;
        }
        case 8: {
          message.pipeline = reader.string();
          break;
        }
        case 10: {
          message.isURL = reader.bool();
          break;
        }
        case 11: {
          message.query = reader.string();
          break;
        }
        case 12: {
          message.isSingleChangeRebuild = reader.bool();
          break;
        }
        case 13: {
          message.optionsId = reader.string();
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetRequestInput message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetRequestInput
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetRequestInput} AssetRequestInput
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetRequestInput.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetRequestInput message.
   * @function verify
   * @memberof AssetRequestInput
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetRequestInput.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.name != null && message.hasOwnProperty('name'))
      if (!$util.isString(message.name)) return 'name: string expected';
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      if (!$util.isString(message.filePath)) return 'filePath: string expected';
    if (message.env != null && message.hasOwnProperty('env'))
      if (!$util.isString(message.env)) return 'env: string expected';
    if (message.isSource != null && message.hasOwnProperty('isSource'))
      if (typeof message.isSource !== 'boolean')
        return 'isSource: boolean expected';
    if (message.canDefer != null && message.hasOwnProperty('canDefer'))
      if (typeof message.canDefer !== 'boolean')
        return 'canDefer: boolean expected';
    if (message.sideEffects != null && message.hasOwnProperty('sideEffects'))
      if (typeof message.sideEffects !== 'boolean')
        return 'sideEffects: boolean expected';
    if (message.code != null && message.hasOwnProperty('code'))
      if (!$util.isString(message.code)) return 'code: string expected';
    if (message.pipeline != null && message.hasOwnProperty('pipeline'))
      if (!$util.isString(message.pipeline)) return 'pipeline: string expected';
    if (message.isURL != null && message.hasOwnProperty('isURL'))
      if (typeof message.isURL !== 'boolean') return 'isURL: boolean expected';
    if (message.query != null && message.hasOwnProperty('query'))
      if (!$util.isString(message.query)) return 'query: string expected';
    if (
      message.isSingleChangeRebuild != null &&
      message.hasOwnProperty('isSingleChangeRebuild')
    )
      if (typeof message.isSingleChangeRebuild !== 'boolean')
        return 'isSingleChangeRebuild: boolean expected';
    if (message.optionsId != null && message.hasOwnProperty('optionsId'))
      if (!$util.isString(message.optionsId))
        return 'optionsId: string expected';
    return null;
  };

  /**
   * Creates an AssetRequestInput message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetRequestInput
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetRequestInput} AssetRequestInput
   */
  AssetRequestInput.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetRequestInput) return object;
    let message = new $root.AssetRequestInput();
    if (object.name != null) message.name = String(object.name);
    if (object.filePath != null) message.filePath = String(object.filePath);
    if (object.env != null) message.env = String(object.env);
    if (object.isSource != null) message.isSource = Boolean(object.isSource);
    if (object.canDefer != null) message.canDefer = Boolean(object.canDefer);
    if (object.sideEffects != null)
      message.sideEffects = Boolean(object.sideEffects);
    if (object.code != null) message.code = String(object.code);
    if (object.pipeline != null) message.pipeline = String(object.pipeline);
    if (object.isURL != null) message.isURL = Boolean(object.isURL);
    if (object.query != null) message.query = String(object.query);
    if (object.isSingleChangeRebuild != null)
      message.isSingleChangeRebuild = Boolean(object.isSingleChangeRebuild);
    if (object.optionsId != null) message.optionsId = String(object.optionsId);
    return message;
  };

  /**
   * Creates a plain object from an AssetRequestInput message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetRequestInput
   * @static
   * @param {AssetRequestInput} message AssetRequestInput
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetRequestInput.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.name = '';
      object.filePath = '';
      object.env = '';
      object.isSource = false;
      object.canDefer = false;
      object.sideEffects = false;
      object.code = '';
      object.pipeline = '';
      object.isURL = false;
      object.query = '';
      object.isSingleChangeRebuild = false;
      object.optionsId = '';
    }
    if (message.name != null && message.hasOwnProperty('name'))
      object.name = message.name;
    if (message.filePath != null && message.hasOwnProperty('filePath'))
      object.filePath = message.filePath;
    if (message.env != null && message.hasOwnProperty('env'))
      object.env = message.env;
    if (message.isSource != null && message.hasOwnProperty('isSource'))
      object.isSource = message.isSource;
    if (message.canDefer != null && message.hasOwnProperty('canDefer'))
      object.canDefer = message.canDefer;
    if (message.sideEffects != null && message.hasOwnProperty('sideEffects'))
      object.sideEffects = message.sideEffects;
    if (message.code != null && message.hasOwnProperty('code'))
      object.code = message.code;
    if (message.pipeline != null && message.hasOwnProperty('pipeline'))
      object.pipeline = message.pipeline;
    if (message.isURL != null && message.hasOwnProperty('isURL'))
      object.isURL = message.isURL;
    if (message.query != null && message.hasOwnProperty('query'))
      object.query = message.query;
    if (
      message.isSingleChangeRebuild != null &&
      message.hasOwnProperty('isSingleChangeRebuild')
    )
      object.isSingleChangeRebuild = message.isSingleChangeRebuild;
    if (message.optionsId != null && message.hasOwnProperty('optionsId'))
      object.optionsId = message.optionsId;
    return object;
  };

  /**
   * Converts this AssetRequestInput to JSON.
   * @function toJSON
   * @memberof AssetRequestInput
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetRequestInput.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetRequestInput
   * @function getTypeUrl
   * @memberof AssetRequestInput
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetRequestInput.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetRequestInput';
  };

  return AssetRequestInput;
})());

export const AssetRequestNode = ($root.AssetRequestNode = (() => {
  /**
   * Properties of an AssetRequestNode.
   * @exports IAssetRequestNode
   * @interface IAssetRequestNode
   * @property {string|null} [id] AssetRequestNode id
   * @property {IAssetRequestInput|null} [input] AssetRequestNode input
   */

  /**
   * Constructs a new AssetRequestNode.
   * @exports AssetRequestNode
   * @classdesc Represents an AssetRequestNode.
   * @implements IAssetRequestNode
   * @constructor
   * @param {IAssetRequestNode=} [properties] Properties to set
   */
  function AssetRequestNode(properties) {
    if (properties)
      for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
        if (properties[keys[i]] != null) this[keys[i]] = properties[keys[i]];
  }

  /**
   * AssetRequestNode id.
   * @member {string} id
   * @memberof AssetRequestNode
   * @instance
   */
  AssetRequestNode.prototype.id = '';

  /**
   * AssetRequestNode input.
   * @member {IAssetRequestInput|null|undefined} input
   * @memberof AssetRequestNode
   * @instance
   */
  AssetRequestNode.prototype.input = null;

  /**
   * Creates a new AssetRequestNode instance using the specified properties.
   * @function create
   * @memberof AssetRequestNode
   * @static
   * @param {IAssetRequestNode=} [properties] Properties to set
   * @returns {AssetRequestNode} AssetRequestNode instance
   */
  AssetRequestNode.create = function create(properties) {
    return new AssetRequestNode(properties);
  };

  /**
   * Encodes the specified AssetRequestNode message. Does not implicitly {@link AssetRequestNode.verify|verify} messages.
   * @function encode
   * @memberof AssetRequestNode
   * @static
   * @param {IAssetRequestNode} message AssetRequestNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetRequestNode.encode = function encode(message, writer) {
    if (!writer) writer = $Writer.create();
    if (message.id != null && Object.hasOwnProperty.call(message, 'id'))
      writer.uint32(/* id 1, wireType 2 =*/ 10).string(message.id);
    if (message.input != null && Object.hasOwnProperty.call(message, 'input'))
      $root.AssetRequestInput.encode(
        message.input,
        writer.uint32(/* id 2, wireType 2 =*/ 18).fork(),
      ).ldelim();
    return writer;
  };

  /**
   * Encodes the specified AssetRequestNode message, length delimited. Does not implicitly {@link AssetRequestNode.verify|verify} messages.
   * @function encodeDelimited
   * @memberof AssetRequestNode
   * @static
   * @param {IAssetRequestNode} message AssetRequestNode message or plain object to encode
   * @param {$protobuf.Writer} [writer] Writer to encode to
   * @returns {$protobuf.Writer} Writer
   */
  AssetRequestNode.encodeDelimited = function encodeDelimited(message, writer) {
    return this.encode(message, writer).ldelim();
  };

  /**
   * Decodes an AssetRequestNode message from the specified reader or buffer.
   * @function decode
   * @memberof AssetRequestNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @param {number} [length] Message length if known beforehand
   * @returns {AssetRequestNode} AssetRequestNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetRequestNode.decode = function decode(reader, length) {
    if (!(reader instanceof $Reader)) reader = $Reader.create(reader);
    let end = length === undefined ? reader.len : reader.pos + length,
      message = new $root.AssetRequestNode();
    while (reader.pos < end) {
      let tag = reader.uint32();
      switch (tag >>> 3) {
        case 1: {
          message.id = reader.string();
          break;
        }
        case 2: {
          message.input = $root.AssetRequestInput.decode(
            reader,
            reader.uint32(),
          );
          break;
        }
        default:
          reader.skipType(tag & 7);
          break;
      }
    }
    return message;
  };

  /**
   * Decodes an AssetRequestNode message from the specified reader or buffer, length delimited.
   * @function decodeDelimited
   * @memberof AssetRequestNode
   * @static
   * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
   * @returns {AssetRequestNode} AssetRequestNode
   * @throws {Error} If the payload is not a reader or valid buffer
   * @throws {$protobuf.util.ProtocolError} If required fields are missing
   */
  AssetRequestNode.decodeDelimited = function decodeDelimited(reader) {
    if (!(reader instanceof $Reader)) reader = new $Reader(reader);
    return this.decode(reader, reader.uint32());
  };

  /**
   * Verifies an AssetRequestNode message.
   * @function verify
   * @memberof AssetRequestNode
   * @static
   * @param {Object.<string,*>} message Plain object to verify
   * @returns {string|null} `null` if valid, otherwise the reason why it is not
   */
  AssetRequestNode.verify = function verify(message) {
    if (typeof message !== 'object' || message === null)
      return 'object expected';
    if (message.id != null && message.hasOwnProperty('id'))
      if (!$util.isString(message.id)) return 'id: string expected';
    if (message.input != null && message.hasOwnProperty('input')) {
      let error = $root.AssetRequestInput.verify(message.input);
      if (error) return 'input.' + error;
    }
    return null;
  };

  /**
   * Creates an AssetRequestNode message from a plain object. Also converts values to their respective internal types.
   * @function fromObject
   * @memberof AssetRequestNode
   * @static
   * @param {Object.<string,*>} object Plain object
   * @returns {AssetRequestNode} AssetRequestNode
   */
  AssetRequestNode.fromObject = function fromObject(object) {
    if (object instanceof $root.AssetRequestNode) return object;
    let message = new $root.AssetRequestNode();
    if (object.id != null) message.id = String(object.id);
    if (object.input != null) {
      if (typeof object.input !== 'object')
        throw TypeError('.AssetRequestNode.input: object expected');
      message.input = $root.AssetRequestInput.fromObject(object.input);
    }
    return message;
  };

  /**
   * Creates a plain object from an AssetRequestNode message. Also converts values to other types if specified.
   * @function toObject
   * @memberof AssetRequestNode
   * @static
   * @param {AssetRequestNode} message AssetRequestNode
   * @param {$protobuf.IConversionOptions} [options] Conversion options
   * @returns {Object.<string,*>} Plain object
   */
  AssetRequestNode.toObject = function toObject(message, options) {
    if (!options) options = {};
    let object = {};
    if (options.defaults) {
      object.id = '';
      object.input = null;
    }
    if (message.id != null && message.hasOwnProperty('id'))
      object.id = message.id;
    if (message.input != null && message.hasOwnProperty('input'))
      object.input = $root.AssetRequestInput.toObject(message.input, options);
    return object;
  };

  /**
   * Converts this AssetRequestNode to JSON.
   * @function toJSON
   * @memberof AssetRequestNode
   * @instance
   * @returns {Object.<string,*>} JSON object
   */
  AssetRequestNode.prototype.toJSON = function toJSON() {
    return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
  };

  /**
   * Gets the default type url for AssetRequestNode
   * @function getTypeUrl
   * @memberof AssetRequestNode
   * @static
   * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
   * @returns {string} The default type url
   */
  AssetRequestNode.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
    if (typeUrlPrefix === undefined) {
      typeUrlPrefix = 'type.googleapis.com';
    }
    return typeUrlPrefix + '/AssetRequestNode';
  };

  return AssetRequestNode;
})());

/**
 * SourceType enum.
 * @exports SourceType
 * @enum {number}
 * @property {number} SOURCE_TYPE_SCRIPT=0 SOURCE_TYPE_SCRIPT value
 * @property {number} SOURCE_TYPE_MODULE=1 SOURCE_TYPE_MODULE value
 */
export const SourceType = ($root.SourceType = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'SOURCE_TYPE_SCRIPT')] = 0;
  values[(valuesById[1] = 'SOURCE_TYPE_MODULE')] = 1;
  return values;
})());

/**
 * OutputFormat enum.
 * @exports OutputFormat
 * @enum {number}
 * @property {number} OUTPUT_FORMAT_ESMODULE=0 OUTPUT_FORMAT_ESMODULE value
 * @property {number} OUTPUT_FORMAT_COMMONJS=1 OUTPUT_FORMAT_COMMONJS value
 * @property {number} OUTPUT_FORMAT_GLOBAL=2 OUTPUT_FORMAT_GLOBAL value
 */
export const OutputFormat = ($root.OutputFormat = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'OUTPUT_FORMAT_ESMODULE')] = 0;
  values[(valuesById[1] = 'OUTPUT_FORMAT_COMMONJS')] = 1;
  values[(valuesById[2] = 'OUTPUT_FORMAT_GLOBAL')] = 2;
  return values;
})());

/**
 * EnvironmentContext enum.
 * @exports EnvironmentContext
 * @enum {number}
 * @property {number} ENVIRONMENT_CONTEXT_BROWSER=0 ENVIRONMENT_CONTEXT_BROWSER value
 * @property {number} ENVIRONMENT_CONTEXT_WEB_WORKER=1 ENVIRONMENT_CONTEXT_WEB_WORKER value
 * @property {number} ENVIRONMENT_CONTEXT_SERVICE_WORKER=2 ENVIRONMENT_CONTEXT_SERVICE_WORKER value
 * @property {number} ENVIRONMENT_CONTEXT_WORKLET=3 ENVIRONMENT_CONTEXT_WORKLET value
 * @property {number} ENVIRONMENT_CONTEXT_NODE=4 ENVIRONMENT_CONTEXT_NODE value
 * @property {number} ENVIRONMENT_CONTEXT_ELECTRON_MAIN=5 ENVIRONMENT_CONTEXT_ELECTRON_MAIN value
 * @property {number} ENVIRONMENT_CONTEXT_ELECTRON_RENDERER=6 ENVIRONMENT_CONTEXT_ELECTRON_RENDERER value
 */
export const EnvironmentContext = ($root.EnvironmentContext = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'ENVIRONMENT_CONTEXT_BROWSER')] = 0;
  values[(valuesById[1] = 'ENVIRONMENT_CONTEXT_WEB_WORKER')] = 1;
  values[(valuesById[2] = 'ENVIRONMENT_CONTEXT_SERVICE_WORKER')] = 2;
  values[(valuesById[3] = 'ENVIRONMENT_CONTEXT_WORKLET')] = 3;
  values[(valuesById[4] = 'ENVIRONMENT_CONTEXT_NODE')] = 4;
  values[(valuesById[5] = 'ENVIRONMENT_CONTEXT_ELECTRON_MAIN')] = 5;
  values[(valuesById[6] = 'ENVIRONMENT_CONTEXT_ELECTRON_RENDERER')] = 6;
  return values;
})());

/**
 * EnvironmentFeature enum.
 * @exports EnvironmentFeature
 * @enum {number}
 * @property {number} ENVIRONMENT_FEATURE_ESMODULES=0 ENVIRONMENT_FEATURE_ESMODULES value
 * @property {number} ENVIRONMENT_FEATURE_DYNAMIC_IMPORT=1 ENVIRONMENT_FEATURE_DYNAMIC_IMPORT value
 * @property {number} ENVIRONMENT_FEATURE_WORKER_MODULE=2 ENVIRONMENT_FEATURE_WORKER_MODULE value
 * @property {number} ENVIRONMENT_FEATURE_SERVICE_WORKER_MODULE=3 ENVIRONMENT_FEATURE_SERVICE_WORKER_MODULE value
 * @property {number} ENVIRONMENT_FEATURE_IMPORT_META_URL=4 ENVIRONMENT_FEATURE_IMPORT_META_URL value
 * @property {number} ENVIRONMENT_FEATURE_ARROW_FUNCTIONS=5 ENVIRONMENT_FEATURE_ARROW_FUNCTIONS value
 * @property {number} ENVIRONMENT_FEATURE_GLOBAL_THIS=6 ENVIRONMENT_FEATURE_GLOBAL_THIS value
 */
export const EnvironmentFeature = ($root.EnvironmentFeature = (() => {
  const valuesById = {},
    values = Object.create(valuesById);
  values[(valuesById[0] = 'ENVIRONMENT_FEATURE_ESMODULES')] = 0;
  values[(valuesById[1] = 'ENVIRONMENT_FEATURE_DYNAMIC_IMPORT')] = 1;
  values[(valuesById[2] = 'ENVIRONMENT_FEATURE_WORKER_MODULE')] = 2;
  values[(valuesById[3] = 'ENVIRONMENT_FEATURE_SERVICE_WORKER_MODULE')] = 3;
  values[(valuesById[4] = 'ENVIRONMENT_FEATURE_IMPORT_META_URL')] = 4;
  values[(valuesById[5] = 'ENVIRONMENT_FEATURE_ARROW_FUNCTIONS')] = 5;
  values[(valuesById[6] = 'ENVIRONMENT_FEATURE_GLOBAL_THIS')] = 6;
  return values;
})());
