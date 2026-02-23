// src/tools/read.ts
import type { CorpusSnapshot } from '../types.js';

export async function read(
  corpus: CorpusSnapshot,
  filePath: string
): Promise<string> {
  const fileEntry = corpus.files[filePath];

  if (!fileEntry) {
    const normalizedPath = filePath.replace(/\\/g, '/');
    const found = corpus.files[normalizedPath];

    if (!found) {
      throw new Error(`File not found: ${filePath}`);
    }

    return found.content;
  }

  return fileEntry.content;
}
