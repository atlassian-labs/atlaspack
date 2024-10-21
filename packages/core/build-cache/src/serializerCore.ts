import v8 from 'v8';

export let serializeRaw: (arg1?: any) => Buffer = v8.serialize;
export let deserializeRaw: (arg1: Buffer) => any = v8.deserialize;
