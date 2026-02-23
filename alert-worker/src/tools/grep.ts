// src/tools/grep.ts
import type { CorpusSnapshot, GrepOptions, GrepResult, MatchDetail } from '../types.js';

export async function grep(
  corpus: CorpusSnapshot,
  pattern: string,
  options: GrepOptions = {}
): Promise<GrepResult[]> {
  const {
    caseInsensitive = false,
    contextLines = 2,
    maxResults = 50,
    maxMatchesPerFile = 5
  } = options;

  let regex: RegExp;
  try {
    const flags = caseInsensitive ? 'gi' : 'g';
    regex = new RegExp(pattern, flags);
  } catch (e) {
    throw new Error(`Invalid regex pattern: ${pattern}`);
  }

  const results: GrepResult[] = [];

  for (const [filePath, fileEntry] of Object.entries(corpus.files)) {
    const lines = fileEntry.content.split('\n');
    const matches: MatchDetail[] = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      regex.lastIndex = 0;

      if (regex.test(line)) {
        const beforeStart = Math.max(0, i - contextLines);
        const afterEnd = Math.min(lines.length, i + contextLines + 1);

        matches.push({
          line: i + 1,
          content: line,
          context: {
            before: lines.slice(beforeStart, i),
            after: lines.slice(i + 1, afterEnd)
          }
        });
      }
    }

    if (matches.length > 0) {
      let score = matches.length;

      if (fileEntry.frontmatter) {
        const fmStr = JSON.stringify(fileEntry.frontmatter).toLowerCase();
        regex.lastIndex = 0;
        if (regex.test(fmStr)) {
          score += 10;
        }
      }

      const earlyMatches = matches.filter(m => m.line < lines.length * 0.2);
      score += earlyMatches.length * 2;

      const truncatedMatches = matches.slice(0, maxMatchesPerFile);

      results.push({
        path: filePath,
        matches: truncatedMatches,
        score
      });
    }
  }

  results.sort((a, b) => b.score - a.score);
  return results.slice(0, maxResults);
}
