// src/context-do.ts
import { DurableObject } from 'cloudflare:workers';
import type { Env, CorpusSnapshot, GrepOptions } from './types.js';
import { grep } from './tools/grep.js';
import { read } from './tools/read.js';
import { list } from './tools/list.js';

export class ContextDO extends DurableObject<Env> {
  private corpus: CorpusSnapshot | null = null;
  private loadPromise: Promise<void> | null = null;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    // Ensure corpus loaded
    if (!this.corpus && !this.loadPromise) {
      this.loadPromise = this.loadCorpusFromR2();
    }
    if (this.loadPromise) {
      await this.loadPromise;
      this.loadPromise = null;
    }

    if (url.pathname === '/grep') {
      const { pattern, options } = await request.json<{
        pattern: string;
        options?: GrepOptions;
      }>();
      const results = await grep(this.corpus!, pattern, options);
      return Response.json(results);
    }

    if (url.pathname === '/read') {
      const { path } = await request.json<{ path: string }>();
      const content = await read(this.corpus!, path);
      return Response.json({ content });
    }

    if (url.pathname === '/list') {
      const { pattern, options } = await request.json<{
        pattern: string;
        options?: any;
      }>();
      const results = await list(this.corpus!, pattern, options);
      return Response.json(results);
    }

    if (url.pathname === '/reload') {
      await this.loadCorpusFromR2();
      return Response.json({ status: 'reloaded', commit: this.corpus?.commit });
    }

    if (url.pathname === '/health') {
      return Response.json({
        loaded: !!this.corpus,
        fileCount: this.corpus?.fileCount ?? 0,
        commit: this.corpus?.commit ?? 'none'
      });
    }

    return new Response('Not found', { status: 404 });
  }

  private async loadCorpusFromR2(): Promise<void> {
    console.log('Loading corpus from R2...');

    const object = await this.env.CORPUS_BUCKET.get('corpus-snapshot.json');
    if (!object) {
      throw new Error('Corpus not found in R2 bucket');
    }

    this.corpus = await object.json<CorpusSnapshot>();
    console.log(`Corpus loaded: ${this.corpus.fileCount} files, commit ${this.corpus.commit}`);
  }
}
