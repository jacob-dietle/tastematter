// src/tools/list.ts
import picomatch from 'picomatch';
import type { CorpusSnapshot, ListOptions, ListResult } from '../types.js';

export async function list(
  corpus: CorpusSnapshot,
  pattern: string,
  options: ListOptions = {}
): Promise<ListResult[]> {
  const {
    directories = true,
    files = true,
    maxResults = 100
  } = options;

  if (!corpus.allPaths) {
    throw new Error('Corpus does not contain allPaths index. Regenerate corpus.');
  }

  let matcher: (path: string) => boolean;
  try {
    matcher = picomatch(pattern, {
      dot: true,
      strictSlashes: false
    });
  } catch (e) {
    throw new Error(`Invalid glob pattern: ${pattern}. ${(e as Error).message}`);
  }

  const matches: ListResult[] = [];

  for (const path of corpus.allPaths) {
    const isDirectory = path.endsWith('/');

    if (isDirectory && !directories) continue;
    if (!isDirectory && !files) continue;

    if (matcher(path)) {
      matches.push({
        path: path,
        type: isDirectory ? 'directory' : 'file',
        depth: calculateDepth(path),
        matchedPattern: pattern
      });

      if (matches.length >= maxResults) {
        console.warn(`List pattern "${pattern}" matched ${maxResults}+ results, truncating`);
        break;
      }
    }
  }

  return matches;
}

function calculateDepth(path: string): number {
  const normalized = path.endsWith('/') ? path.slice(0, -1) : path;
  if (!normalized || normalized === '.') return 0;
  return normalized.split('/').length - 1;
}
