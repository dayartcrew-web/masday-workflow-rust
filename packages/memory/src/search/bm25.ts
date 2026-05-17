/**
 * BM25 keyword search implementation.
 *
 * Uses the Okapi BM25 algorithm for text relevance scoring.
 */
export interface BM25Document {
  id: string;
  content: string;
}

export interface BM25Result {
  id: string;
  score: number;
}

const DEFAULT_K1 = 1.5;
const DEFAULT_B = 0.75;

export class BM25Search {
  private readonly k1: number;
  private readonly b: number;
  private documents: BM25Document[] = [];
  private termFrequencies: Map<string, Map<string, number>> = new Map();
  private documentLengths: Map<string, number> = new Map();
  private avgDocLength: number = 0;
  private documentCount: number = 0;
  private inverseDocumentFrequency: Map<string, number> = new Map();

  constructor(k1: number = DEFAULT_K1, b: number = DEFAULT_B) {
    this.k1 = k1;
    this.b = b;
  }

  /** Index documents for searching. */
  index(documents: BM25Document[]): void {
    this.documents = documents;
    this.documentCount = documents.length;
    this.termFrequencies = new Map();
    this.documentLengths = new Map();

    let totalLength = 0;
    const docFrequency = new Map<string, number>();

    for (const doc of documents) {
      const tokens = this.tokenize(doc.content);
      this.documentLengths.set(doc.id, tokens.length);
      totalLength += tokens.length;

      const tf = new Map<string, number>();
      for (const token of tokens) {
        tf.set(token, (tf.get(token) ?? 0) + 1);
      }
      this.termFrequencies.set(doc.id, tf);

      // Track which documents contain each term
      const uniqueTokens = new Set(tokens);
      for (const token of uniqueTokens) {
        docFrequency.set(token, (docFrequency.get(token) ?? 0) + 1);
      }
    }

    this.avgDocLength = this.documentCount > 0 ? totalLength / this.documentCount : 0;

    // Compute IDF
    this.inverseDocumentFrequency = new Map();
    for (const [term, freq] of docFrequency) {
      this.inverseDocumentFrequency.set(
        term,
        Math.log((this.documentCount - freq + 0.5) / (freq + 0.5) + 1)
      );
    }
  }

  /** Search documents using BM25 scoring. */
  search(query: string, limit: number = 10): BM25Result[] {
    const queryTokens = this.tokenize(query);
    const scores = new Map<string, number>();

    for (const queryToken of queryTokens) {
      const idf = this.inverseDocumentFrequency.get(queryToken) ?? 0;
      if (idf <= 0) continue;

      for (const doc of this.documents) {
        const tf = this.termFrequencies.get(doc.id)?.get(queryToken) ?? 0;
        if (tf === 0) continue;

        const docLength = this.documentLengths.get(doc.id) ?? 0;
        const numerator = tf * (this.k1 + 1);
        const denominator = tf + this.k1 * (1 - this.b + this.b * (docLength / (this.avgDocLength || 1)));
        const score = idf * (numerator / denominator);

        scores.set(doc.id, (scores.get(doc.id) ?? 0) + score);
      }
    }

    const results: BM25Result[] = [];
    for (const [id, score] of scores) {
      results.push({ id, score });
    }

    results.sort((a, b) => b.score - a.score);
    return results.slice(0, limit);
  }

  /** Tokenize text into lowercase terms. */
  private tokenize(text: string): string[] {
    return text
      .toLowerCase()
      .split(/\W+/)
      .filter(token => token.length > 1);
  }
}
