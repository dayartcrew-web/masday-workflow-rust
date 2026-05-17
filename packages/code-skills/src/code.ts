import { z } from "zod";
import { createLogger } from "@mcp-rebuild/core";
import { SemanticSearcher } from "@mcp-rebuild/intelligence";

const logger = createLogger("CodeSkills");

// Lazy-initialised singleton for SemanticSearcher
let searcherInstance: SemanticSearcher | null = null;

function getSearcher(): SemanticSearcher {
  if (!searcherInstance) {
    searcherInstance = new SemanticSearcher();
    logger.info("SemanticSearcher lazy-initialised");
  }
  return searcherInstance;
}

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

export const codeSearchSchema = z.object({
  query: z.string(),
  extensions: z.array(z.string()).optional(),
  pathPattern: z.string().optional(),
  maxSize: z.number().optional(),
});

export type CodeSearchInput = z.infer<typeof codeSearchSchema>;

export const codeSearchOutputSchema = z.object({
  results: z.array(
    z.object({
      filePath: z.string(),
      line: z.number(),
      match: z.string(),
      context: z.string(),
      score: z.number(),
    }),
  ),
});

export type CodeSearchOutput = z.infer<typeof codeSearchOutputSchema>;

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export async function runCodeSearch(
  input: unknown,
): Promise<CodeSearchOutput> {
  const { query, extensions, pathPattern, maxSize } = codeSearchSchema.parse(input);

  const searcher = getSearcher();

  const results = await searcher.search({
    query,
    patterns: [],
    fileFilter: {
      extensions,
      pathPattern,
      maxSize,
    },
  });

  logger.info(`Code search: "${query}" returned ${results.length} results`);
  return { results };
}
