// @mcp-rebuild/memory - 4-Layer Memory System
//
// Layers:
// 1. Working Memory    - in-process RAM keyed by session ID
// 2. Episodic Memory   - ring buffer of last N messages per session
// 3. Long-Term Memory  - file-based store with scoring (similarity*0.6 + recency*0.15 + importance*0.15 + usage*0.1)
// 4. Knowledge Graph   - Dijkstra, BFS, auto-linking with Jaccard similarity

export { WorkingMemory } from './working.js';
export type { SessionState } from './working.js';

export { EpisodicMemory, setEpisodicDb } from './episodic.js';
export type { ChatMessage } from './episodic.js';

export { MemoryStore } from './store.js';
export type { MemoryStoreConfig, SearchOptions, PruneOptions, EmbeddingProvider } from './store.js';

export { GraphStore, setGraphDb } from './graph.js';
export type { GraphStoreConfig } from './graph.js';

export { ScoringEngine } from './scoring.js';
export type { ScoringWeights, ScoredMemory } from './scoring.js';

export { MemoryClassifier } from './classifier.js';
export type { ILLMProvider, ClassificationResult } from './classifier.js';

export { ReflectionEngine } from './reflection.js';
export type { ReflectionConfig } from './reflection.js';

export { EmbeddingService, MockEmbeddingService } from './embedding.js';
export type { EmbeddingConfig } from './embedding.js';

export { TripleStreamSearch } from './search/index.js';
export { BM25Search } from './search/bm25.js';
export type { TripleStreamConfig, VectorProvider, KGProvider } from './search/index.js';
export type { BM25Document, BM25Result } from './search/bm25.js';
