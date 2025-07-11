import v8 from 'v8';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const serializeRaw: (_val: any) => Buffer = v8.serialize;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const deserializeRaw: (_buff: Buffer) => any = v8.deserialize;
