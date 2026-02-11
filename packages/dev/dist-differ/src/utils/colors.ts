/**
 * ANSI color codes for terminal output
 * Colors are disabled when output is piped (not a TTY)
 */

export interface Colors {
  reset: string;
  red: string;
  green: string;
  yellow: string;
  cyan: string;
  dim: string;
}

export function getColors(): Colors {
  // Check if output is being piped (not a TTY)
  const useColors = process.stdout.isTTY === true;

  return {
    reset: useColors ? '\x1b[0m' : '',
    red: useColors ? '\x1b[31m' : '',
    green: useColors ? '\x1b[32m' : '',
    yellow: useColors ? '\x1b[33m' : '',
    cyan: useColors ? '\x1b[36m' : '',
    dim: useColors ? '\x1b[2m' : '',
  };
}
