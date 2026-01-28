declare module 'dotenv' {
  export interface DotenvParseOutput {
    [key: string]: string;
  }

  export interface DotenvConfigOptions {
    path?: string;
    encoding?: string;
  }

  export interface DotenvConfigOutput {
    parsed?: DotenvParseOutput;
  }

  export function config(options?: DotenvConfigOptions): DotenvConfigOutput;
  export function parse(src: string | Buffer): DotenvParseOutput;
}

declare module 'dotenv-expand' {
  import type {DotenvParseOutput} from 'dotenv';

  export interface DotenvExpandOptions {
    parsed?: DotenvParseOutput;
    ignoreProcessEnv?: boolean;
  }

  export interface DotenvExpandOutput {
    parsed?: DotenvParseOutput;
    error?: Error;
  }

  export default function expand(
    options: DotenvExpandOptions,
  ): DotenvExpandOutput;
}
