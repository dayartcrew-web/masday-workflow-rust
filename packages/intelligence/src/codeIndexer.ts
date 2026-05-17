/**
 * CodeIndexer - file/code indexing with embedding generation.
 *
 * Enhanced to support:
 * 1. Repository scanning and symbol extraction
 * 2. Chunk-based indexing for granular search
 * 3. Embedding generation for semantic search
 * 4. Storage of indexed chunks via StorageBackend
 */

import { promises as fs } from 'fs';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import { createLogger } from '@mcp-rebuild/core';
import { EventBus } from '@mcp-rebuild/core';
import type { FileMetadata, IndexedRepository, CodeSymbol, DependencyEdge, IndexedChunk, IndexResult } from './types.js';

const logger = createLogger('intelligence:indexer');

/** Embedding provider for generating vector embeddings. */
export interface IndexerEmbeddingProvider {
  embed(text: string): Promise<number[]>;
  embedBatch(texts: string[]): Promise<number[][]>;
}

const DEFAULT_CHUNK_SIZE = 50; // lines per chunk
const DEFAULT_CHUNK_OVERLAP = 5; // overlapping lines between chunks

export class CodeIndexer {
  private readonly eventBus: EventBus;
  private readonly ignorePatterns: string[];
  private readonly embeddingProvider: IndexerEmbeddingProvider | null;
  private readonly chunkSize: number;
  private readonly chunkOverlap: number;
  private readonly chunkStore: Map<string, IndexedChunk> = new Map();

  constructor(
    eventBus: EventBus,
    ignorePatterns: string[] = [],
    embeddingProvider?: IndexerEmbeddingProvider,
    options?: { chunkSize?: number; chunkOverlap?: number },
  ) {
    this.eventBus = eventBus;
    this.ignorePatterns = ignorePatterns;
    this.embeddingProvider = embeddingProvider ?? null;
    this.chunkSize = options?.chunkSize ?? DEFAULT_CHUNK_SIZE;
    this.chunkOverlap = options?.chunkOverlap ?? DEFAULT_CHUNK_OVERLAP;
  }

  /** Get all indexed chunks. */
  getChunks(): IndexedChunk[] {
    return Array.from(this.chunkStore.values());
  }

  /** Get a chunk by ID. */
  getChunk(id: string): IndexedChunk | undefined {
    return this.chunkStore.get(id);
  }

  /**
   * Index a repository: scan files, extract symbols, analyze dependencies.
   * Returns the indexed repository metadata.
   */
  async indexRepository(rootPath: string): Promise<IndexedRepository> {
    logger.info(`Indexing repository: ${rootPath}`);

    const startTime = Date.now();

    const files = new Map<string, FileMetadata>();
    const symbols = new Map<string, CodeSymbol[]>();
    const dependencies = new Map<string, DependencyEdge[]>();

    const allFiles = await this.scanDirectory(rootPath);

    for (const filePath of allFiles) {
      const metadata = await this.getFileMetadata(filePath, rootPath);
      files.set(filePath, metadata);

      const fileSymbols = await this.extractSymbols(filePath);
      if (fileSymbols.length > 0) {
        symbols.set(filePath, fileSymbols);
      }

      logger.debug(`Indexed: ${filePath} (${metadata.lineCount} lines, ${fileSymbols.length} symbols)`);
    }

    await this.analyzeDependencies(allFiles, files, dependencies);

    const duration = Date.now() - startTime;
    const fileCount = files.size;

    logger.info(`Indexing complete: ${fileCount} files in ${duration}ms`);

    const indexedRepo: IndexedRepository = {
      files,
      symbols,
      dependencies,
      indexedAt: new Date(),
    };

    this.eventBus.emit('repository.indexed', {
      rootPath,
      fileCount,
      duration,
    });

    return indexedRepo;
  }

  /**
   * Index code files with chunk-based embedding generation.
   * Creates granular chunks and optionally generates embeddings.
   */
  async indexWithEmbeddings(rootPath: string): Promise<IndexResult> {
    const startTime = Date.now();

    const allFiles = await this.scanDirectory(rootPath);
    const allChunks: IndexedChunk[] = [];

    for (const filePath of allFiles) {
      const ext = path.extname(filePath).replace(/^\./, '');
      const language = this.getLanguage(ext);

      if (!language) continue;

      const content = await fs.readFile(filePath, 'utf-8');
      const lines = content.split('\n');
      const relativePath = path.relative(rootPath, filePath);

      // Create overlapping chunks
      const chunks = this.createChunks(relativePath, content, lines, language);
      allChunks.push(...chunks);
    }

    // Generate embeddings for all chunks
    if (this.embeddingProvider && allChunks.length > 0) {
      const texts = allChunks.map(c => c.content);
      const batchResults = await this.embeddingProvider.embedBatch(texts);

      for (let i = 0; i < allChunks.length; i++) {
        allChunks[i] = {
          ...allChunks[i],
          embedding: batchResults[i] ?? [],
        };
      }
    }

    // Store chunks
    for (const chunk of allChunks) {
      this.chunkStore.set(chunk.id, chunk);
    }

    const duration = Date.now() - startTime;
    const totalTokens = allChunks.reduce((sum, c) => sum + c.content.split(/\s+/).length, 0);

    logger.info({
      totalChunks: allChunks.length,
      totalTokens,
      durationMs: duration,
    }, 'Indexing with embeddings complete');

    this.eventBus.emit('file.indexed', {
      rootPath,
      chunkCount: allChunks.length,
      duration,
    });

    return {
      totalChunks: allChunks.length,
      totalTokens,
      durationMs: duration,
    };
  }

  /** Create overlapping chunks from file content. */
  private createChunks(
    filePath: string,
    _content: string,
    lines: string[],
    language: string,
  ): IndexedChunk[] {
    const chunks: IndexedChunk[] = [];

    for (let startLine = 0; startLine < lines.length; startLine += this.chunkSize - this.chunkOverlap) {
      const endLine = Math.min(startLine + this.chunkSize, lines.length);
      const chunkLines = lines.slice(startLine, endLine);
      const chunkContent = chunkLines.join('\n');

      if (chunkContent.trim().length === 0) continue;

      chunks.push({
        id: uuidv4(),
        filePath,
        content: chunkContent,
        embedding: [], // Will be filled by embedding provider
        language,
        startLine: startLine + 1,
        endLine,
        indexedAt: Date.now(),
      });

      if (endLine >= lines.length) break;
    }

    return chunks;
  }

  /** Get language name from file extension. */
  private getLanguage(ext: string): string | null {
    const mapping: Record<string, string> = {
      ts: 'typescript',
      tsx: 'typescript',
      js: 'javascript',
      jsx: 'javascript',
      py: 'python',
      rs: 'rust',
      go: 'go',
      java: 'java',
      json: 'json',
      yaml: 'yaml',
      yml: 'yaml',
      md: 'markdown',
      sql: 'sql',
    };

    return mapping[ext] ?? null;
  }

  // --- Legacy Repository Indexing Methods ---

  private async scanDirectory(dirPath: string): Promise<string[]> {
    const files: string[] = [];

    let entries;
    try {
      entries = await fs.readdir(dirPath, { withFileTypes: true });
    } catch {
      return files;
    }

    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);

      // Check ignore patterns
      if (this.ignorePatterns.some(pattern => entry.name === pattern || entry.name.startsWith('.'))) {
        continue;
      }

      if (entry.isDirectory()) {
        const subFiles = await this.scanDirectory(fullPath);
        files.push(...subFiles);
      } else if (entry.isFile()) {
        files.push(fullPath);
      }
    }

    return files;
  }

  private async getFileMetadata(filePath: string, rootPath: string): Promise<FileMetadata> {
    const stats = await fs.stat(filePath);
    const relativePath = path.relative(rootPath, filePath);
    const ext = path.extname(filePath).replace(/^\./, '');

    const lines = await this.countLines(filePath);

    const metadata: FileMetadata = {
      path: relativePath,
      size: stats.size,
      extension: ext || 'none',
      lastModified: stats.mtime,
      isDirectory: stats.isDirectory(),
      lineCount: lines,
    };

    return metadata;
  }

  private async countLines(filePath: string): Promise<number> {
    const content = await fs.readFile(filePath, 'utf-8');
    return content.split('\n').length;
  }

  private async extractSymbols(filePath: string): Promise<CodeSymbol[]> {
    const content = await fs.readFile(filePath, 'utf-8');
    const symbols: CodeSymbol[] = [];
    const ext = path.extname(filePath).replace(/^\./, '');

    const patterns: Record<string, RegExp[]> = {
      ts: [
        /interface\s+(\w+)/g,
        /type\s+(\w+)/g,
        /class\s+(\w+)/g,
        /enum\s+(\w+)/g,
        /function\s+(\w+)/g,
        /(?:const|let|var)\s+(\w+)/g,
        /export\s+(?:default\s+)?(?:function|class|const|let|var)\s+(\w+)/g,
      ],
      js: [
        /function\s+(\w+)/g,
        /class\s+(\w+)/g,
        /(?:const|let|var)\s+(\w+)/g,
      ],
      py: [
        /def\s+(\w+)/g,
        /class\s+(\w+)/g,
      ],
    };

    const langPatterns = patterns[ext] || [];

    for (const regex of langPatterns) {
      regex.lastIndex = 0;
      let match;
      while ((match = regex.exec(content)) !== null) {
        const before = content.substring(0, match.index);
        const lineNumber = before.split('\n').length;
        const column = match.index - before.lastIndexOf('\n');

        symbols.push({
          name: match[1],
          type: this.getSymbolType(match[0]),
          filePath,
          line: lineNumber,
          column,
          exported: match[0].startsWith('export'),
        });
      }
    }

    return symbols;
  }

  private getSymbolType(match: string): CodeSymbol['type'] {
    const lower = match.toLowerCase();

    if (lower.startsWith('interface')) return 'interface';
    if (lower.startsWith('type ')) return 'type';
    if (lower.startsWith('class')) return 'class';
    if (lower.startsWith('enum')) return 'enum';
    if (lower.startsWith('function') || lower.startsWith('def ')) return 'function';
    if (lower.startsWith('const') || lower.startsWith('let') || lower.startsWith('var')) return 'variable';
    if (lower.startsWith('export')) {
      if (lower.includes('function')) return 'function';
      if (lower.includes('class')) return 'class';
      if (lower.includes('const') || lower.includes('let')) return 'variable';
    }

    return 'variable';
  }

  private async analyzeDependencies(
    allFiles: string[],
    _files: Map<string, FileMetadata>,
    dependencies: Map<string, DependencyEdge[]>,
  ): Promise<void> {
    logger.info('Analyzing dependencies...');

    for (const filePath of allFiles) {
      const fileContent = await fs.readFile(filePath, 'utf-8');

      const importMatches = [
        ...fileContent.matchAll(/import\s+['"]([^'"]+)['"]/g),
        ...fileContent.matchAll(/from\s+['"]([^'"]+)['"]/g),
      ];

      for (const match of importMatches) {
        const importedModule = match[1];
        const strength = this.calculateDependencyStrength(match[0]);

        const dependency: DependencyEdge = {
          from: filePath,
          to: importedModule,
          type: 'import',
          strength,
        };

        const existingDeps = dependencies.get(filePath) || [];
        dependencies.set(filePath, [...existingDeps, dependency]);
      }
    }
  }

  private calculateDependencyStrength(importKeyword: string): number {
    if (/^import\s/.test(importKeyword)) return 9;
    if (/^from\s/.test(importKeyword)) return 8;
    return 5;
  }
}
