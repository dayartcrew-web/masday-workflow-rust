// ============================================================
// Token estimation — server-side approximation for MCP tool calls
// ============================================================

let tokenizer: { encode(text: string): number[] } | null = null;
let tokenizerLoaded = false;

function loadTokenizer(): void {
  if (tokenizerLoaded) return;
  tokenizerLoaded = true;
  try {
    // gpt-tokenizer is a peer of the OpenAI tokenizer; ~80% accurate for Claude
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const mod = require('gpt-tokenizer');
    tokenizer = { encode: (text: string) => mod.encode(text) };
  } catch {
    tokenizer = null;
  }
}

/**
 * Estimate the number of tokens in a string.
 * Uses gpt-tokenizer when available (~80% accurate for Claude),
 * falls back to chars/4 heuristic otherwise.
 */
export function estimateTokens(input: string): number {
  if (!input) return 0;
  loadTokenizer();
  if (tokenizer) {
    return tokenizer.encode(input).length;
  }
  return Math.ceil(input.length / 4);
}
