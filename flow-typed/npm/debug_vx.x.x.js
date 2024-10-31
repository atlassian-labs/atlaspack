// flow-typed signature: 9dd28a0a0a1d5e68bae91223d7b6050b
// flow-typed version: <<STUB>>/debug_v4.3.4/flow_v0.184.0

declare module 'debug' {
  declare type DebugFactory = (tag: string) => (msg: string, ...args: Array<any>) => void;
  declare module.exports: DebugFactory;
}
