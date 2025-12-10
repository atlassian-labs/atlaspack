/**
 * Type definitions for diff operations
 */

/**
 * Type of diff entry: equal (unchanged), remove (deleted), or add (added)
 */
export type DiffEntryType = 'equal' | 'remove' | 'add';

/**
 * Represents a single line in a diff
 */

export interface DiffEntry {
  type: DiffEntryType;
  line: string;
  lineNum1?: number;
  lineNum2?: number;
}

/**
 * Information about a file in a directory
 */
export interface FileInfo {
  /** Absolute path to the file */
  fullPath: string;
  /** Relative path from the base directory */
  relativePath: string;
  /** Filename only */
  filename: string;
  /** File size in bytes */
  size: number;
}

/**
 * A pair of matched files from two directories
 */
export interface MatchedPair {
  file1: FileInfo;
  file2: FileInfo;
  /** Prefix extracted from filename (before hash) */
  prefix: string;
  /** Directory path where files were found */
  dirPath: string;
}

/**
 * Ambiguous file match when multiple files could match
 */
export interface AmbiguousMatch {
  /** Prefix that matched */
  prefix: string;
  /** Directory path */
  dirPath: string;
  /** Files in first directory */
  files1: FileInfo[];
  /** Files in second directory */
  files2: FileInfo[];
}

/**
 * Result of matching files between two directories
 */
export interface MatchResult {
  /** Successfully matched file pairs */
  matched: MatchedPair[];
  /** Ambiguous matches that couldn't be resolved */
  ambiguous: AmbiguousMatch[];
}

/**
 * Result of a diff operation
 */
export interface DiffResult {
  /** Number of hunks (change blocks) in the diff */
  hunkCount: number;
  /** Whether there are any changes */
  hasChanges: boolean;
}
