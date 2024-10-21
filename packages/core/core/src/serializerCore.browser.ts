import {Buffer} from 'buffer';
import * as msgpackr from 'msgpackr';

let encoder = new msgpackr.Encoder({structuredClone: true});

export let serializeRaw: (arg1?: any) => Buffer = (v) =>
  Buffer.from(encoder.encode(v));
export let deserializeRaw: (arg1: Buffer) => any = (v) => encoder.decode(v);
