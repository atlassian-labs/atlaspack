declare module 'slice-ansi' {
  function sliceAnsi(input: string, start: number, end?: number): string;

  export = sliceAnsi;
}
