/**
 * Type definitions for diff operations
 */

export type DiffEntryType = 'equal' | 'remove' | 'add';

export interface DiffEntry {
  type: DiffEntryType;
  line: string;
  lineNum1?: number;
  lineNum2?: number;
}

export interface FileInfo {
  fullPath: string;
  relativePath: string;
  filename: string;
  size: number;
}

export interface MatchedPair {
  file1: FileInfo;
  file2: FileInfo;
  prefix: string;
  dirPath: string;
}

export interface AmbiguousMatch {
  prefix: string;
  dirPath: string;
  files1: FileInfo[];
  files2: FileInfo[];
}

export interface MatchResult {
  matched: MatchedPair[];
  ambiguous: AmbiguousMatch[];
}

export interface DiffResult {
  hunkCount: number;
  hasChanges: boolean;
}
