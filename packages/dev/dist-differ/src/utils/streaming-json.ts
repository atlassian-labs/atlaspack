/* eslint-disable no-console */
import * as stream from 'stream';

/**
 * A streaming JSON writer that writes JSON incrementally to avoid memory issues
 * with large objects. This is useful when JSON.stringify would fail due to size.
 */
export class StreamingJsonWriter {
  private output: stream.Writable;
  private indentLevel: number = 0;
  private indentString: string;
  private isFirstItem: boolean = true;
  private isInArray: boolean = false;
  private isInObject: boolean = false;

  constructor(output: stream.Writable = process.stdout, indent: number = 2) {
    this.output = output;
    this.indentString = ' '.repeat(indent);
  }

  private write(str: string): void {
    this.output.write(str);
  }

  private indent(): string {
    return this.indentString.repeat(this.indentLevel);
  }

  private newline(): void {
    this.write('\n');
  }

  /**
   * Starts writing a JSON object
   */
  startObject(): void {
    if (!this.isFirstItem && this.isInArray) {
      this.write(',');
      this.newline();
    }
    this.write('{');
    this.newline();
    this.indentLevel++;
    this.isFirstItem = true;
    this.isInObject = true;
  }

  /**
   * Ends the current JSON object
   */
  endObject(): void {
    this.indentLevel--;
    this.newline();
    this.write(this.indent() + '}');
    this.isInObject = false;
    this.isFirstItem = false;
  }

  /**
   * Starts writing a JSON array
   */
  startArray(): void {
    if (!this.isFirstItem && this.isInObject) {
      this.write(',');
      this.newline();
    }
    this.write('[');
    this.newline();
    this.indentLevel++;
    this.isFirstItem = true;
    this.isInArray = true;
  }

  /**
   * Ends the current JSON array
   */
  endArray(): void {
    this.indentLevel--;
    this.newline();
    this.write(this.indent() + ']');
    this.isInArray = false;
    this.isFirstItem = false;
  }

  /**
   * Writes a property key
   */
  propertyKey(key: string): void {
    if (!this.isFirstItem) {
      this.write(',');
      this.newline();
    }
    this.write(this.indent() + JSON.stringify(key) + ': ');
    this.isFirstItem = false;
  }

  /**
   * Writes a property with a value
   */
  property(key: string, value: unknown): void {
    this.propertyKey(key);
    this.value(value);
  }

  /**
   * Writes a JSON value (string, number, boolean, null)
   */
  value(val: unknown): void {
    if (val === null || val === undefined) {
      this.write('null');
    } else if (typeof val === 'string') {
      this.write(JSON.stringify(val));
    } else if (typeof val === 'number' || typeof val === 'boolean') {
      this.write(String(val));
    } else if (Array.isArray(val)) {
      // For arrays, we'll stringify them directly (they should be small)
      this.write(JSON.stringify(val));
    } else if (typeof val === 'object') {
      // For objects, we'll stringify them directly (they should be small)
      // If they're too large, the caller should use startObject/endObject
      this.write(JSON.stringify(val));
    } else {
      this.write(JSON.stringify(String(val)));
    }
    this.isFirstItem = false;
  }

  /**
   * Writes an array item
   */
  arrayItem(item: unknown): void {
    if (!this.isFirstItem) {
      this.write(',');
      this.newline();
    }
    this.write(this.indent());
    this.value(item);
    this.isFirstItem = false;
  }

  /**
   * Flushes the output stream
   */
  flush(): void {
    if (this.output instanceof stream.Writable && 'flush' in this.output) {
      (this.output as any).flush();
    }
  }

  /**
   * Ends the stream
   */
  end(): void {
    if (this.output instanceof stream.Writable) {
      this.output.end();
    }
  }
}
