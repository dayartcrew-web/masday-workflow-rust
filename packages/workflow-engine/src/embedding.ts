/**
 * Embedding providers (msd-mcp business logic)
 */

import { env } from "@mcp-rebuild/shared-utils";

export interface EmbeddingProvider {
  embed(text: string): Promise<number[]>;
  embedBatch(texts: string[]): Promise<number[][]>;
}

export class MockEmbeddingProvider implements EmbeddingProvider {
  private dims: number;

  constructor(opts?: { dimensions?: number }) {
    this.dims = opts?.dimensions ?? env.EMBEDDING_DIMENSIONS;
  }

  async embed(text: string): Promise<number[]> {
    const arr = new Array(this.dims).fill(0);
    arr[0] = text.length;
    arr[1] = [...text].reduce((a, c) => a + c.charCodeAt(0), 0) % 997;
    return arr;
  }

  async embedBatch(texts: string[]): Promise<number[][]> {
    return Promise.all(texts.map((t) => this.embed(t)));
  }
}

export class OpenAIEmbeddingProvider implements EmbeddingProvider {
  private apiKey: string;
  private model: string;
  private baseUrl: string;

  constructor(opts?: {
    apiKey?: string;
    model?: string;
    baseUrl?: string;
  }) {
    this.apiKey = opts?.apiKey ?? env.OPENAI_API_KEY;
    this.model = opts?.model ?? env.EMBEDDING_MODEL;
    this.baseUrl = opts?.baseUrl ?? env.OPENAI_BASE_URL;

    if (!this.apiKey) {
      throw new Error(
        "OPENAI_API_KEY is required for OpenAIEmbeddingProvider. " +
          "Set it in .env or pass via constructor.",
      );
    }
  }

  async embed(text: string): Promise<number[]> {
    const results = await this.embedBatch([text]);
    return results[0];
  }

  async embedBatch(texts: string[]): Promise<number[][]> {
    const response = await fetch(`${this.baseUrl}/embeddings`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${this.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: this.model,
        input: texts,
      }),
    });

    if (!response.ok) {
      const body = await response.text();
      throw new Error(
        `OpenAI embeddings API error (${response.status}): ${body}`,
      );
    }

    const data = (await response.json()) as {
      data: Array<{ embedding: number[] }>;
    };

    const sorted = data.data.sort(
      (a, b) => a.embedding.length - b.embedding.length,
    );

    return sorted.map((d) => d.embedding);
  }
}

export class FastEmbedEmbeddingProvider implements EmbeddingProvider {
  private modelPromise: Promise<unknown> | null = null;
  private modelId: string;
  private cacheDir: string;
  private timeoutMs: number;

  constructor(opts?: { model?: string; cacheDir?: string; timeoutMs?: number }) {
    this.modelId = opts?.model ?? env.EMBEDDING_MODEL;
    this.cacheDir = opts?.cacheDir ?? "local_cache";
    this.timeoutMs = opts?.timeoutMs ?? 30_000;
  }

  private async getModel(): Promise<unknown> {
    if (!this.modelPromise) {
      this.modelPromise = (async () => {
        const { createRequire } = await import("node:module");
        const require = createRequire(import.meta.url);
        const { FlagEmbedding, EmbeddingModel } = require("fastembed") as {
          FlagEmbedding: { init(opts: { model: string; cacheDir: string }): Promise<unknown> };
          EmbeddingModel: Record<string, string>;
        };
        const id = this.modelId || EmbeddingModel.BGEBaseENV15;
        return FlagEmbedding.init({ model: id, cacheDir: this.cacheDir });
      })();
    }
    const result = await Promise.race([
      this.modelPromise,
      new Promise<null>((resolve) => setTimeout(() => resolve(null), this.timeoutMs)),
    ]);
    if (!result) {
      this.modelPromise = null;
      throw new Error(`FastEmbed model init timed out after ${this.timeoutMs}ms`);
    }
    return result;
  }

  async embed(text: string): Promise<number[]> {
    const m = await this.getModel();
    return (m as { queryEmbed(text: string): Promise<number[]> }).queryEmbed(text);
  }

  async embedBatch(texts: string[]): Promise<number[][]> {
    return Promise.all(texts.map((t) => this.embed(t)));
  }
}

export function createEmbeddingProvider(
  provider?: string,
): EmbeddingProvider {
  const p = provider ?? env.EMBEDDING_PROVIDER;

  switch (p) {
    case "fastembed":
      return new FastEmbedEmbeddingProvider();
    case "ollama":
      return new OpenAIEmbeddingProvider({
        apiKey: "ollama",
        baseUrl: env.OLLAMA_BASE_URL + "/v1",
      });
    case "openai":
      return new OpenAIEmbeddingProvider();
    case "mock":
    default:
      return new MockEmbeddingProvider();
  }
}
