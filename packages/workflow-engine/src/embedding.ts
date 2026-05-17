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

export function createEmbeddingProvider(
  provider?: string,
): EmbeddingProvider {
  const p = provider ?? env.EMBEDDING_PROVIDER;

  switch (p) {
    case "openai":
      return new OpenAIEmbeddingProvider();
    case "mock":
    default:
      return new MockEmbeddingProvider();
  }
}
