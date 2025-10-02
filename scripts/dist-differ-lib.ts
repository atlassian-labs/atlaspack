import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {execSync} from 'child_process';

// ANSI color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  gray: '\x1b[90m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
} as const;

export interface FileInfo {
  relativePath: string;
  fullPath: string;
  size: number;
  normalizedPath?: string; // Path with content hash stripped
}

export interface MatchingOptions {
  sizeThresholdPercent: number; // Percentage threshold for size matching (default 5%)
}

/**
 * Strip content hash from file path
 * e.g. "MyBundle.123456.js" -> "MyBundle.js"
 * e.g. "assets/bundle.a1b2c3d4.css" -> "assets/bundle.css"
 * e.g. "file.hash123.min.js" -> "file.min.js"
 */
export function stripContentHash(filePath: string): string {
  const dir = path.dirname(filePath);
  const basename = path.basename(filePath);
  const ext = path.extname(basename);
  const nameWithoutExt = path.basename(basename, ext);

  // Match patterns like: name.hash.ext or name.hash.min.ext where hash is 6+ alphanumeric characters
  // This regex looks for a dot followed by 6+ alphanumeric characters, either at the end or followed by common suffixes
  const hashPattern = /\.([a-z0-9]{6,})(?=\.|$)/i;
  const normalizedName = nameWithoutExt.replace(hashPattern, '');

  return path.join(dir, normalizedName + ext);
}

/**
 * Check if two file sizes are within the specified threshold
 */
export function isWithinSizeThreshold(
  size1: number,
  size2: number,
  thresholdPercent: number,
): boolean {
  if (size1 === 0 && size2 === 0) return true;
  if (size1 === 0 || size2 === 0) return false;

  const larger = Math.max(size1, size2);
  const smaller = Math.min(size1, size2);
  const percentDiff = ((larger - smaller) / larger) * 100;

  return percentDiff <= thresholdPercent;
}

export function getAllFiles(dir: string, baseDir: string = dir): FileInfo[] {
  const files: FileInfo[] = [];

  try {
    const entries = fs.readdirSync(dir, {withFileTypes: true});

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (fullPath.endsWith('.js.map')) {
        // Skip .js.map files
        continue;
      }

      if (entry.isDirectory()) {
        files.push(...getAllFiles(fullPath, baseDir));
      } else if (entry.isFile()) {
        const relativePath = path.relative(baseDir, fullPath);
        const stats = fs.statSync(fullPath);
        files.push({
          relativePath,
          fullPath,
          size: stats.size,
          normalizedPath: stripContentHash(relativePath),
        });
      }
    }
  } catch (error) {
    console.error(`Error reading directory ${dir}:`, error);
    process.exit(1);
  }

  return files;
}

/**
 * Find the best matching file for a given file from another directory
 * Prioritizes: exact path > normalized path + size similarity > normalized path > size similarity
 */
export function findMatchingFile(
  targetFile: FileInfo,
  candidateFiles: FileInfo[],
  options: MatchingOptions,
): FileInfo | null {
  // 1. Try exact path match first
  const exactMatch = candidateFiles.find(
    (f) => f.relativePath === targetFile.relativePath,
  );
  if (exactMatch) return exactMatch;

  // 2. Get all candidates with same normalized path
  const normalizedMatches = candidateFiles.filter(
    (f) => f.normalizedPath === targetFile.normalizedPath,
  );

  if (normalizedMatches.length === 0) {
    return null; // No possible matches
  }

  if (normalizedMatches.length === 1) {
    return normalizedMatches[0]; // Only one option
  }

  // 3. If multiple normalized matches, prefer the one with closest size
  let bestMatch = normalizedMatches[0];
  let bestSizeDiff = Math.abs(targetFile.size - bestMatch.size);

  for (const candidate of normalizedMatches.slice(1)) {
    const sizeDiff = Math.abs(targetFile.size - candidate.size);
    if (sizeDiff < bestSizeDiff) {
      bestMatch = candidate;
      bestSizeDiff = sizeDiff;
    }
  }

  return bestMatch;
}

/**
 * Simple formatter for minified JavaScript to improve diff readability
 * Adds newlines after common patterns without full parsing for performance
 */
export function formatMinifiedJs(content: string): string {
  // Quick check if this looks like minified JS (long lines, minimal whitespace)
  const lines = content.split('\n');
  const avgLineLength = content.length / lines.length;
  const hasLongLines = lines.some((line) => line.length > 500);

  // Only format if it looks minified (average line length > 200 or has very long lines)
  if (avgLineLength < 200 && !hasLongLines) {
    return content; // Already formatted or not JS
  }

  // Add newlines after common JS patterns for better diffing
  let formatted = content
    // Add newlines after semicolons (but not in strings or regexes)
    .replace(/;(?=(?:[^"'`]|"[^"]*"|'[^']*'|`[^`]*`)*$)/gm, ';\n')
    // Add newlines after opening braces
    .replace(/{/g, '{\n')
    // Add newlines before closing braces
    .replace(/}/g, '\n}')
    // Add newlines after commas in object/array literals (heuristic)
    .replace(/,(?=(?:[^"'`]|"[^"]*"|'[^']*'|`[^`]*`)*[}\]])/g, ',\n')
    // Clean up multiple consecutive newlines
    .replace(/\n{3,}/g, '\n\n')
    // Clean up leading/trailing whitespace on lines
    .replace(/^[ \t]+|[ \t]+$/gm, '');

  return formatted;
}

/**
 * Analyze diff lines to find identifier-only changes (variable renames)
 * Returns null if changes are substantial, or a mapping of renamed identifiers
 */
function analyzeIdentifierChanges(line1: string, line2: string): Map<string, string> | null {
  // Skip if lines are identical or very different in length
  if (line1 === line2) return new Map();
  if (Math.abs(line1.length - line2.length) > 10) return null;
  
  // Tokenize both lines into meaningful parts
  // This regex splits on word boundaries, operators, and punctuation while preserving them
  const tokenize = (line: string) => line.match(/\w+|[^\w\s]/g) || [];
  
  const tokens1 = tokenize(line1);
  const tokens2 = tokenize(line2);
  
  // If token count differs significantly, probably not just identifier changes
  if (tokens1.length !== tokens2.length) return null;
  
  const identifierMap = new Map<string, string>();
  const identifierPattern = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/;
  
  for (let i = 0; i < tokens1.length; i++) {
    const token1 = tokens1[i];
    const token2 = tokens2[i];
    
    if (token1 === token2) {
      continue; // Same token, no change
    }
    
    // Both tokens should be valid identifiers
    if (!identifierPattern.test(token1) || !identifierPattern.test(token2)) {
      return null; // Non-identifier change detected
    }
    
    // Check for consistent mapping
    if (identifierMap.has(token1)) {
      if (identifierMap.get(token1) !== token2) {
        return null; // Inconsistent mapping
      }
    } else {
      identifierMap.set(token1, token2);
    }
  }
  
  return identifierMap;
}

/**
 * Filter diff output to remove lines that only differ by identifier names
 * Returns filtered diff and a summary of ignored changes
 */
function filterIdentifierOnlyChanges(diffOutput: string): {
  filteredDiff: string;
  ignoredChanges: Map<string, string>;
  removedLines: number;
} {
  const lines = diffOutput.split('\n');
  const filteredLines: string[] = [];
  const allIdentifierChanges = new Map<string, string>();
  let removedLines = 0;
  let i = 0;
  
  while (i < lines.length) {
    const line = lines[i];
    
    // Look for diff chunks starting with @@ 
    if (line.startsWith('@@')) {
      filteredLines.push(line);
      i++;
      
      // Process the chunk
      const chunkStart = i;
      const removedLinesInChunk: string[] = [];
      const addedLinesInChunk: string[] = [];
      
      // Collect all - lines and + lines in this chunk
      while (i < lines.length && !lines[i].startsWith('@@')) {
        const currentLine = lines[i];
        if (currentLine.startsWith('-')) {
          const content = currentLine.substring(1).trim();
          if (content.length >= 3) { // Skip very short lines
            removedLinesInChunk.push(currentLine);
          } else {
            filteredLines.push(currentLine);
          }
        } else if (currentLine.startsWith('+')) {
          const content = currentLine.substring(1).trim();
          if (content.length >= 3) { // Skip very short lines
            addedLinesInChunk.push(currentLine);
          } else {
            filteredLines.push(currentLine);
          }
        } else {
          // Context line (starts with space) or other line
          filteredLines.push(currentLine);
        }
        i++;
      }
      
      // Try to match removed and added lines by identifier patterns
      const usedAdded = new Set<number>();
      const filteredRemoved: string[] = [];
      const filteredAdded: string[] = [];
      
      for (const removedLine of removedLinesInChunk) {
        const removedContent = removedLine.substring(1).trim();
        let bestMatch = -1;
        let bestChanges: Map<string, string> | null = null;
        
        // Find the best matching added line
        for (let j = 0; j < addedLinesInChunk.length; j++) {
          if (usedAdded.has(j)) continue;
          
          const addedContent = addedLinesInChunk[j].substring(1).trim();
          const identifierChanges = analyzeIdentifierChanges(removedContent, addedContent);
          
          if (identifierChanges !== null && identifierChanges.size > 0) {
            bestMatch = j;
            bestChanges = identifierChanges;
            break; // Take first valid match
          }
        }
        
        if (bestMatch >= 0 && bestChanges) {
          // Found a match - this is an identifier-only change
          usedAdded.add(bestMatch);
          for (const [oldId, newId] of bestChanges) {
            allIdentifierChanges.set(oldId, newId);
          }
          removedLines += 2;
        } else {
          // No match - keep the removed line
          filteredRemoved.push(removedLine);
        }
      }
      
      // Add any unmatched added lines
      for (let j = 0; j < addedLinesInChunk.length; j++) {
        if (!usedAdded.has(j)) {
          filteredAdded.push(addedLinesInChunk[j]);
        }
      }
      
      // Add the remaining lines to output
      filteredLines.push(...filteredRemoved);
      filteredLines.push(...filteredAdded);
      
      continue; // i is already advanced
    }
    
    // Keep non-chunk lines (headers, etc.)
    filteredLines.push(line);
    i++;
  }
  
  return {
    filteredDiff: filteredLines.join('\n'),
    ignoredChanges: allIdentifierChanges,
    removedLines
  };
}

/**
 * Colorize diff output for better readability
 */
function colorizeDiff(diffOutput: string): string {
  const lines = diffOutput.split('\n');
  const colorizedLines = lines.map(line => {
    if (line.startsWith('---')) {
      // File header (old file)
      return `${colors.bold}${colors.red}${line}${colors.reset}`;
    } else if (line.startsWith('+++')) {
      // File header (new file)  
      return `${colors.bold}${colors.green}${line}${colors.reset}`;
    } else if (line.startsWith('@@')) {
      // Hunk header
      return `${colors.bold}${colors.cyan}${line}${colors.reset}`;
    } else if (line.startsWith('-')) {
      // Removed line
      return `${colors.red}${line}${colors.reset}`;
    } else if (line.startsWith('+')) {
      // Added line
      return `${colors.green}${line}${colors.reset}`;
    } else if (line.startsWith(' ')) {
      // Context line
      return `${colors.gray}${line}${colors.reset}`;
    } else {
      // Other lines (unchanged)
      return line;
    }
  });
  
  return colorizedLines.join('\n');
}

/**
 * Check if color output should be disabled
 */
function shouldDisableColors(): boolean {
  // Force colors if FORCE_COLOR is set
  if (process.env.FORCE_COLOR) {
    return false;
  }
  
  // Disable colors if:
  // 1. NO_COLOR environment variable is set
  // 2. Not running in a TTY (e.g., piped to file) and not forced
  // 3. CI environment (some CI systems don't handle colors well)
  return !!(
    process.env.NO_COLOR ||
    !process.stdout.isTTY ||
    process.env.CI
  );
}

/**
 * Apply colors to diff output if appropriate
 */
function maybeColorizeDiff(diffOutput: string): string {
  if (shouldDisableColors()) {
    return diffOutput;
  }
  return colorizeDiff(diffOutput);
}

/**
 * Check if a file appears to be JavaScript based on extension and content
 */
export function isJavaScriptFile(filePath: string, content?: string): boolean {
  const jsExtensions = ['.js', '.mjs', '.jsx', '.ts', '.tsx'];
  const hasJsExtension = jsExtensions.some((ext) =>
    filePath.toLowerCase().endsWith(ext),
  );

  if (hasJsExtension) return true;

  // Also check content for JS patterns if provided
  if (content) {
    // Look for common JS patterns
    const jsPatterns = [
      /function\s*\(/,
      /var\s+\w+\s*=/,
      /const\s+\w+\s*=/,
      /let\s+\w+\s*=/,
      /=\s*function\s*\(/,
      /=>\s*{/,
      /require\s*\(/,
      /module\.exports/,
      /export\s+(default\s+)?/,
    ];

    return jsPatterns.some((pattern) =>
      pattern.test(content.substring(0, 1000)),
    );
  }

  return false;
}

export function compareFileContents(
  file1: string,
  file2: string,
): string | null {
  try {
    const content1 = fs.readFileSync(file1);
    const content2 = fs.readFileSync(file2);

    if (Buffer.compare(content1, content2) === 0) {
      return null; // Files are identical
    }

    // Check if these are JavaScript files that might benefit from formatting
    const content1Str = content1.toString('utf8');
    const content2Str = content2.toString('utf8');

    const isJs1 = isJavaScriptFile(file1, content1Str);
    const isJs2 = isJavaScriptFile(file2, content2Str);

    // If both files are JavaScript and look minified, format them for better diffing
    if (isJs1 && isJs2) {
      const formatted1 = formatMinifiedJs(content1Str);
      const formatted2 = formatMinifiedJs(content2Str);

      // Only use formatted versions if formatting actually changed the content
      // (avoids unnecessary work on already-formatted files)
      if (formatted1 !== content1Str || formatted2 !== content2Str) {
        try {
          // Create temporary files with formatted content for diffing
          const tempDir = fs.mkdtempSync(
            path.join(os.tmpdir(), 'dist-differ-'),
          );
          const tempFile1 = path.join(tempDir, 'file1' + path.extname(file1));
          const tempFile2 = path.join(tempDir, 'file2' + path.extname(file2));

          fs.writeFileSync(tempFile1, formatted1);
          fs.writeFileSync(tempFile2, formatted2);

          try {
            const diffOutput = execSync(`diff -u "${tempFile1}" "${tempFile2}"`, {
              encoding: 'utf8',
              maxBuffer: 50 * 1024 * 1024, // 50MB buffer
            });

            // Clean up temp files
            fs.rmSync(tempDir, {recursive: true, force: true});

            // Replace temp file paths with original paths in diff output
            let processedDiff = diffOutput
              .replace(
                new RegExp(
                  tempFile1.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
                  'g',
                ),
                file1,
              )
              .replace(
                new RegExp(
                  tempFile2.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
                  'g',
                ),
                file2,
              );
            
            // Filter out identifier-only changes for JavaScript files
            const filtered = filterIdentifierOnlyChanges(processedDiff);
            
            if (filtered.removedLines > 0) {
              // Add summary of ignored identifier changes
              if (filtered.ignoredChanges.size > 0) {
                let result = filtered.filteredDiff;
                result += '\n' + '='.repeat(40) + '\n';
                
                const ignoreMessage = shouldDisableColors() 
                  ? `📝 Ignored ${filtered.removedLines} lines with only identifier changes:\n`
                  : `${colors.yellow}📝 Ignored ${filtered.removedLines} lines with only identifier changes:${colors.reset}\n`;
                result += ignoreMessage;
                
                const changes = Array.from(filtered.ignoredChanges.entries())
                  .slice(0, 10) // Show max 10 examples
                  .map(([old, new_]) => shouldDisableColors() 
                    ? `${old} → ${new_}`
                    : `${colors.red}${old}${colors.reset} → ${colors.green}${new_}${colors.reset}`
                  )
                  .join(', ');
                
                result += `   ${changes}`;
                if (filtered.ignoredChanges.size > 10) {
                  result += ` ... (+${filtered.ignoredChanges.size - 10} more)`;
                }
                result += '\n';
                
                // If after filtering there are only header lines left, show a simplified message
                const meaningfulLines = filtered.filteredDiff.split('\n')
                  .filter(line => !line.startsWith('---') && !line.startsWith('+++') && !line.startsWith('@@') && line.trim().length > 0);
                
                if (meaningfulLines.length <= 2) {
                  return `Files differ only by identifier names (variable/function renames).\n${result}`;
                }
                
                return maybeColorizeDiff(result);
              }
              
              return maybeColorizeDiff(filtered.filteredDiff);
            }
            
            return maybeColorizeDiff(processedDiff);
          } catch (diffError) {
            // Clean up temp files
            fs.rmSync(tempDir, {recursive: true, force: true});

            // diff command returns non-zero exit code when files differ
            if (diffError instanceof Error && 'stdout' in diffError) {
              let output = (diffError as any).stdout;
              output = output
                .replace(
                  new RegExp(
                    tempFile1.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
                    'g',
                  ),
                  file1,
                )
                .replace(
                  new RegExp(
                    tempFile2.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
                    'g',
                  ),
                  file2,
                );
              
              // Filter identifier-only changes for catch block too
              const filtered = filterIdentifierOnlyChanges(output);
              if (filtered.removedLines > 0) {
                let result = filtered.filteredDiff;
                if (filtered.ignoredChanges.size > 0) {
                  result += '\n' + '='.repeat(40) + '\n';
                  const ignoreMessage = shouldDisableColors() 
                    ? `📝 Ignored ${filtered.removedLines} lines with only identifier changes:\n`
                    : `${colors.yellow}📝 Ignored ${filtered.removedLines} lines with only identifier changes:${colors.reset}\n`;
                  result += ignoreMessage;
                  const changes = Array.from(filtered.ignoredChanges.entries())
                    .slice(0, 10)
                    .map(([old, new_]) => shouldDisableColors() 
                      ? `${old} → ${new_}`
                      : `${colors.red}${old}${colors.reset} → ${colors.green}${new_}${colors.reset}`
                    )
                    .join(', ');
                  result += `   ${changes}`;
                  if (filtered.ignoredChanges.size > 10) {
                    result += ` ... (+${filtered.ignoredChanges.size - 10} more)`;
                  }
                  result += '\n';
                }
                return maybeColorizeDiff(result);
              }
              return maybeColorizeDiff(output);
            }
          }
        } catch (tempError) {
          console.warn(
            'Failed to create formatted diff, falling back to original files',
          );
        }
      }
    }

    // Fallback to original diff method
    try {
      const diffOutput = execSync(`diff -u "${file1}" "${file2}"`, {
        encoding: 'utf8',
        maxBuffer: 50 * 1024 * 1024, // 50MB buffer
      });
      
      // Apply identifier filtering for JavaScript files even in fallback
      if (isJs1 && isJs2) {
        const filtered = filterIdentifierOnlyChanges(diffOutput);
        if (filtered.removedLines > 0) {
          let result = filtered.filteredDiff;
          if (filtered.ignoredChanges.size > 0) {
            result += '\n' + '='.repeat(40) + '\n';
            const ignoreMessage = shouldDisableColors() 
              ? `📝 Ignored ${filtered.removedLines} lines with only identifier changes:\n`
              : `${colors.yellow}📝 Ignored ${filtered.removedLines} lines with only identifier changes:${colors.reset}\n`;
            result += ignoreMessage;
            const changes = Array.from(filtered.ignoredChanges.entries())
              .slice(0, 10)
              .map(([old, new_]) => shouldDisableColors() 
                ? `${old} → ${new_}`
                : `${colors.red}${old}${colors.reset} → ${colors.green}${new_}${colors.reset}`
              )
              .join(', ');
            result += `   ${changes}`;
            if (filtered.ignoredChanges.size > 10) {
              result += ` ... (+${filtered.ignoredChanges.size - 10} more)`;
            }
            result += '\n';
          }
          return maybeColorizeDiff(result);
        }
      }
      
      return maybeColorizeDiff(diffOutput);
    } catch (diffError) {
      // diff command returns non-zero exit code when files differ
      if (diffError instanceof Error && 'stdout' in diffError) {
        let output = (diffError as any).stdout;
        
        // Apply identifier filtering for JavaScript files even in error case
        if (isJs1 && isJs2) {
          const filtered = filterIdentifierOnlyChanges(output);
          if (filtered.removedLines > 0) {
            let result = filtered.filteredDiff;
            if (filtered.ignoredChanges.size > 0) {
              result += '\n' + '='.repeat(40) + '\n';
              const ignoreMessage = shouldDisableColors() 
                ? `📝 Ignored ${filtered.removedLines} lines with only identifier changes:\n`
                : `${colors.yellow}📝 Ignored ${filtered.removedLines} lines with only identifier changes:${colors.reset}\n`;
              result += ignoreMessage;
              const changes = Array.from(filtered.ignoredChanges.entries())
                .slice(0, 10)
                .map(([old, new_]) => shouldDisableColors() 
                  ? `${old} → ${new_}`
                  : `${colors.red}${old}${colors.reset} → ${colors.green}${new_}${colors.reset}`
                )
                .join(', ');
              result += `   ${changes}`;
              if (filtered.ignoredChanges.size > 10) {
                result += ` ... (+${filtered.ignoredChanges.size - 10} more)`;
              }
              result += '\n';
            }
            return maybeColorizeDiff(result);
          }
        }
        
        return maybeColorizeDiff(output);
      }
      return `Files differ but unable to generate readable diff. File sizes: ${content1.length} vs ${content2.length} bytes`;
    }
  } catch (error) {
    return `Error comparing files: ${error}`;
  }
}
