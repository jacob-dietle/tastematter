<!-- src/lib/components/SessionFileTree.svelte -->
<script lang="ts">
  import type { SessionFile, DirectoryNode } from '$lib/types';

  let {
    files,
    colorScale,
    onFileClick
  }: {
    files: SessionFile[];
    colorScale: (count: number) => string;
    onFileClick: (filePath: string) => void;
  } = $props();

  // Build directory tree from flat file list
  function buildTree(files: SessionFile[]): DirectoryNode[] {
    const root: DirectoryNode = {
      name: '',
      path: '',
      type: 'directory',
      access_count: 0,
      children: [],
      expanded: true
    };

    for (const file of files) {
      const parts = file.file_path.split('/');
      let current = root;

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isFile = i === parts.length - 1;
        const path = parts.slice(0, i + 1).join('/');

        let child = current.children?.find(c => c.name === part);

        if (!child) {
          child = {
            name: part,
            path,
            type: isFile ? 'file' : 'directory',
            access_count: isFile ? file.access_count : 0,
            children: isFile ? undefined : [],
            expanded: true
          };
          current.children?.push(child);
        }

        if (!isFile) {
          child.access_count += file.access_count;
          current = child;
        }
      }
    }

    // Sort: directories first, then by access count
    function sortNodes(nodes: DirectoryNode[]): DirectoryNode[] {
      return nodes.sort((a, b) => {
        if (a.type !== b.type) return a.type === 'directory' ? -1 : 1;
        return b.access_count - a.access_count;
      }).map(n => ({
        ...n,
        children: n.children ? sortNodes(n.children) : undefined
      }));
    }

    return sortNodes(root.children || []);
  }

  const tree = $derived(buildTree(files));

  let expandedDirs = $state<Set<string>>(new Set());

  function toggleDir(path: string) {
    const newSet = new Set(expandedDirs);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    expandedDirs = newSet;
  }

  function isDirExpanded(path: string): boolean {
    return expandedDirs.has(path) || path.split('/').length <= 2;
  }
</script>

<div class="file-tree" data-testid="file-tree">
  {#snippet renderNode(node: DirectoryNode, depth: number)}
    <div
      class="tree-node"
      class:directory={node.type === 'directory'}
      class:file={node.type === 'file'}
      style="padding-left: {depth * 16}px"
    >
      {#if node.type === 'directory'}
        <button
          class="dir-toggle"
          onclick={() => toggleDir(node.path)}
        >
          {isDirExpanded(node.path) ? '▼' : '▶'}
        </button>
        <span class="dir-name">{node.name}/</span>
        <span class="dir-count">({node.access_count})</span>
      {:else}
        <span
          class="file-dot"
          style="background: {colorScale(node.access_count)}"
        ></span>
        <button
          class="file-name"
          onclick={() => onFileClick(node.path)}
        >
          {node.name}
        </button>
        <span class="file-count">{node.access_count}</span>
      {/if}
    </div>

    {#if node.type === 'directory' && isDirExpanded(node.path) && node.children}
      {#each node.children as child (child.path)}
        {@render renderNode(child, depth + 1)}
      {/each}
    {/if}
  {/snippet}

  {#each tree as node (node.path)}
    {@render renderNode(node, 0)}
  {/each}
</div>

<style>
  .file-tree {
    font-family: monospace;
    font-size: 0.85em;
  }

  .tree-node {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 0;
  }

  .dir-toggle {
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 0.8em;
    color: var(--text-muted, #6a737d);
  }

  .dir-name {
    color: var(--text-primary, #24292e);
    font-weight: 500;
  }

  .dir-count {
    color: var(--text-muted, #6a737d);
    font-size: 0.9em;
  }

  .file-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    margin-left: 16px;
  }

  .file-name {
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-primary, #24292e);
  }

  .file-name:hover {
    color: var(--color-link, #0366d6);
    text-decoration: underline;
  }

  .file-count {
    color: var(--text-muted, #6a737d);
    margin-left: auto;
  }
</style>
