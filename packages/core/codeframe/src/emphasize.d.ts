declare module 'emphasize' {
  interface HighlightResult {
    relevance: number;
    language: string;
    value: string;
  }

  function highlight(language: string, code: string): HighlightResult;
  function highlightAuto(code: string): HighlightResult;

  const emphasize: {
    highlight: typeof highlight;
    highlightAuto: typeof highlightAuto;
  };

  export = emphasize;
}
