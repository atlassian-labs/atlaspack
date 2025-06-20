import type { Glob } from "./Glob.mts";
import type { FilePath } from "./FilePath.mts";

export type GlobInvalidation = {
  glob: Glob;
};

export type FileInvalidation = {
  filePath: FilePath;
};

export type FileAboveInvalidation = {
  fileName: string;
  aboveFilePath: FilePath;
};

export type FileCreateInvalidation = FileInvalidation | GlobInvalidation | FileAboveInvalidation;
