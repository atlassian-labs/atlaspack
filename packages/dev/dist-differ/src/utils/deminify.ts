/* eslint-disable no-console */
import * as fs from 'fs';

/**
 * Reads a file and splits it on semicolons and commas to "de-minify" it
 * for easier diff comparison
 */
export function readAndDeminify(filePath: string): string[] | null {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    // Split on semicolons and commas, keeping track of which delimiter was used
    const lines: string[] = [];
    let currentLine = '';

    for (let i = 0; i < content.length; i++) {
      const char = content[i];
      if (char === ';' || char === ',') {
        currentLine += char;
        if (currentLine.trim().length > 0) {
          lines.push(currentLine);
        }
        currentLine = '';
      } else {
        currentLine += char;
      }
    }

    // Add any remaining content
    if (currentLine.trim().length > 0) {
      lines.push(currentLine);
    }

    return lines;
  } catch (error) {
    console.error(
      `Error reading file ${filePath}: ${error instanceof Error ? error.message : String(error)}`,
    );
    process.exitCode = 1;
    return null;
  }
}
