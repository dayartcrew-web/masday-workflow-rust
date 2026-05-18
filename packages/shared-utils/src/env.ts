import { config } from "dotenv";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { z } from "zod";

// Always load .env from the project root, regardless of cwd.
// This allows MCP servers to work when spawned from any project directory.
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
config({ path: resolve(__dirname, "../../../.env") });

export const env = z
  .object({
    // ── Database ──
    DATABASE_URL: z.string().min(1).default("sqlite://local"),
    DB_USER: z.string().default("postgres"),
    DB_PASSWORD: z.string().default("postgres"),
    DB_PORT: z.coerce.number().int().default(5432),
    DB_HOST: z.string().default("localhost"),
    DB_NAME: z.string().default("claude_agent_platform"),

    // ── Embedding ──
    EMBEDDING_PROVIDER: z.enum(["mock", "openai", "ollama"]).default("mock"),
    EMBEDDING_DIMENSIONS: z.coerce.number().int().min(1).default(768),
    EMBEDDING_MODEL: z.string().default("nomic-embed-text"),

    // ── OpenAI-compatible API (used when EMBEDDING_PROVIDER=openai) ──
    OPENAI_API_KEY: z.string().default(""),
    OPENAI_BASE_URL: z.string().default("https://api.openai.com/v1"),

    // ── Ollama (used when EMBEDDING_PROVIDER=ollama) ──
    OLLAMA_BASE_URL: z.string().default("http://localhost:11434"),
  })
  .parse(process.env);

// Typed env access — import { env } from "@mcp-rebuild/shared-utils"
// Usage: env.DATABASE_URL, env.EMBEDDING_PROVIDER, etc.
