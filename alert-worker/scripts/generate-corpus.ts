// scripts/generate-corpus.ts
// Generate corpus snapshot from a directory for context publishing
import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import matter from 'gray-matter';

interface CorpusSnapshot {
  version: string;
  commit: string;
  fileCount: number;
  totalSize: number;
  generatedAt: string;
  files: Record<string, FileEntry>;
  allPaths: string[];
}

interface FileEntry {
  path: string;
  content: string;
  size: number;
  frontmatter?: any;
}

function buildPathIndex(files: Record<string, FileEntry>): string[] {
  const allPaths = new Set<string>();

  for (const filePath of Object.keys(files)) {
    allPaths.add(filePath);

    const parts = filePath.split('/');
    for (let i = 1; i < parts.length; i++) {
      const dirPath = parts.slice(0, i).join('/') + '/';
      allPaths.add(dirPath);
    }
  }

  return Array.from(allPaths).sort();
}

async function generateCorpus(repoPath: string, globs?: string[]): Promise<CorpusSnapshot> {
  const files: Record<string, FileEntry> = {};
  let totalSize = 0;

  const commit = execSync('git rev-parse HEAD', { cwd: repoPath })
    .toString()
    .trim();

  // Use provided globs or default to *.md *.yaml
  const filePatterns = globs && globs.length > 0
    ? globs.map(g => `"${g}"`).join(' ')
    : '"*.md" "*.yaml"';

  console.error(`Using git ls-files with patterns: ${filePatterns}`);
  const gitFiles = execSync(`git ls-files ${filePatterns}`, { cwd: repoPath })
    .toString()
    .trim()
    .split('\n')
    .filter(f => f.length > 0);

  console.error(`Found ${gitFiles.length} tracked files`);

  for (const relativePath of gitFiles) {
    const fullPath = path.join(repoPath, relativePath);

    if (!fs.existsSync(fullPath)) {
      console.warn(`Warning: ${relativePath} listed by git but not found on disk`);
      continue;
    }

    const content = fs.readFileSync(fullPath, 'utf-8');
    const normalizedPath = relativePath.replace(/\\/g, '/');
    const size = Buffer.byteLength(content, 'utf-8');

    let frontmatter: any = undefined;
    if (normalizedPath.endsWith('.md')) {
      try {
        const parsed = matter(content);
        if (Object.keys(parsed.data).length > 0) {
          frontmatter = parsed.data;
        }
      } catch (e) {
        console.error(`Failed to parse frontmatter in ${normalizedPath}:`, e);
      }
    }

    files[normalizedPath] = {
      path: normalizedPath,
      content,
      size,
      frontmatter
    };

    totalSize += size;
  }

  const allPaths = buildPathIndex(files);

  return {
    version: '1.0',
    commit,
    fileCount: Object.keys(files).length,
    totalSize,
    generatedAt: new Date().toISOString(),
    files,
    allPaths
  };
}

// CLI usage
const args = process.argv.slice(2);
const repoPath = args[0];
const globs = args.slice(1);

if (!repoPath) {
  console.error('Usage: tsx scripts/generate-corpus.ts <repo-path> [glob1] [glob2] ...');
  console.error('Example: tsx scripts/generate-corpus.ts /path/to/repo "*.md" "*.yaml"');
  process.exit(1);
}

if (!fs.existsSync(repoPath)) {
  console.error(`Error: Repository path does not exist: ${repoPath}`);
  process.exit(1);
}

generateCorpus(repoPath, globs.length > 0 ? globs : undefined).then(corpus => {
  console.log(JSON.stringify(corpus, null, 2));
}).catch(error => {
  console.error('Error generating corpus:', error);
  process.exit(1);
});
