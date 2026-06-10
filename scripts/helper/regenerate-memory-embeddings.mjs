/**
 * Regenerate embeddings for all memories with null embedding field.
 * Uses Ollama (nomic-embed-text) to generate 768-dim vectors via pgvector.
 *
 * Usage:
 *   node scripts/regenerate-memory-embeddings.mjs              # Supabase (remote)
 *   node scripts/regenerate-memory-embeddings.mjs --local      # Local PostgreSQL
 *   node scripts/regenerate-memory-embeddings.mjs --batch-size 20  # Custom batch size
 */

import { createHash } from 'crypto';
import { Client } from 'pg';

// ── Config ──────────────────────────────────────────────────────────────────

const IS_LOCAL = process.argv.includes('--local');
const BATCH_SIZE = parseInt(process.argv.find(a => a.startsWith('--batch-size='))?.split('=')[1]) || 20;

const DB_CONFIG = IS_LOCAL
  ? {
      host: 'localhost',
      port: 54341,
      user: 'postgres',
      password: 'postgres',
      database: 'masday_workflow',
    }
  : {
      host: 'db.gvjevnfmxiqpikvxiunc.supabase.co',
      port: 5432,
      user: 'postgres',
      password: 'pU335NIco29IwqsQ',
      database: 'postgres',
    };

const OLLAMA_URL = process.env.OLLAMA_URL || 'http://localhost:11434';
const OLLAMA_MODEL = process.env.OLLAMA_MODEL || 'nomic-embed-text:latest';
const EXPECTED_DIMS = 768;

// ── Ollama Embedding ────────────────────────────────────────────────────────

/**
 * Generate embedding using Ollama API.
 * Tries new endpoint (/api/embed) first, falls back to legacy (/api/embeddings).
 */
async function getEmbedding(text) {
  const truncated = text.length > 2000 ? text.substring(0, 2000) : text;

  // Try new Ollama API: POST /api/embed with "input"
  try {
    const res = await fetch(`${OLLAMA_URL}/api/embed`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ model: OLLAMA_MODEL, input: truncated }),
    });
    if (res.ok) {
      const data = await res.json();
      if (data.embeddings?.[0]?.length === EXPECTED_DIMS) {
        return data.embeddings[0];
      }
    }
  } catch {}

  // Legacy Ollama API: POST /api/embeddings with "prompt"
  const res = await fetch(`${OLLAMA_URL}/api/embeddings`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: OLLAMA_MODEL, prompt: truncated }),
  });
  if (!res.ok) {
    throw new Error(`Ollama returned ${res.status}: ${await res.text()}`);
  }
  const data = await res.json();
  if (!data.embedding || data.embedding.length !== EXPECTED_DIMS) {
    throw new Error(`Unexpected embedding dimensions: ${data.embedding?.length}`);
  }
  return data.embedding;
}

/**
 * Generate embedding input from memory summary + content.
 * Concatenates summary and first 500 chars of content for optimal semantic coverage.
 */
function buildEmbeddingInput(summary, content) {
  const contentSnippet = (content || '').substring(0, 500);
  return `${summary}\n${contentSnippet}`.trim();
}

// ── Fingerprint ─────────────────────────────────────────────────────────────

/**
 * Generate deterministic SHA-256 fingerprint for memory content.
 * Used as a content hash to detect changes.
 */
function makeContentFingerprint(summary, content) {
  return createHash('sha256').update(`${summary}||${content}`).digest('hex');
}

// ── Main ────────────────────────────────────────────────────────────────────

async function main() {
  const client = new Client(DB_CONFIG);
  await client.connect();
  console.log(`Connected to ${IS_LOCAL ? 'local' : 'Supabase'} PostgreSQL`);

  // Check pgvector extension
  const { rows: extRows } = await client.query(`
    SELECT extname, extversion FROM pg_extension WHERE extname = 'vector'
  `);
  if (extRows.length === 0) {
    console.error('ERROR: pgvector extension not installed. Run: CREATE EXTENSION vector;');
    process.exit(1);
  }
  console.log(`pgvector v${extRows[0].extversion} available`);

  // Count memories needing embeddings
  const { rows: stats } = await client.query(`
    SELECT
      COUNT(*) as total,
      COUNT(embedding) as has_embedding,
      COUNT(*) - COUNT(embedding) as needs_embedding
    FROM "Memory"
  `);
  console.log(`Memory stats: ${stats[0].total} total, ${stats[0].has_embedding} with embedding, ${stats[0].needs_embedding} need embedding`);

  if (parseInt(stats[0].needs_embedding) === 0) {
    console.log('All memories already have embeddings. Nothing to do.');
    await client.end();
    return;
  }

  // Check Ollama connectivity
  try {
    const res = await fetch(`${OLLAMA_URL}/api/tags`);
    const data = await res.json();
    const models = data.models?.map(m => m.name) || [];
    console.log(`Ollama connected, models: ${models.join(', ')}`);
    if (!models.some(m => m.startsWith('nomic-embed-text'))) {
      console.error('ERROR: nomic-embed-text model not found in Ollama. Pull it first:');
      console.error('  ollama pull nomic-embed-text');
      process.exit(1);
    }
  } catch (e) {
    console.error(`ERROR: Cannot connect to Ollama at ${OLLAMA_URL}: ${e.message}`);
    process.exit(1);
  }

  // Fetch memories needing embeddings in batches
  // No OFFSET — each batch re-queries WHERE embedding IS NULL since we update in-place
  let totalUpdated = 0;
  let totalFailed = 0;
  let totalSkipped = 0;

  while (true) {
    const { rows: memories } = await client.query(`
      SELECT id, summary, content, "memoryType", "source"
      FROM "Memory"
      WHERE embedding IS NULL
      ORDER BY "createdAt" ASC
      LIMIT $1
    `, [BATCH_SIZE]);

    if (memories.length === 0) break;

    console.log(`\nProcessing batch: ${memories.length} memories (remaining: ${totalUpdated} done)`);

    for (const mem of memories) {
      try {
        // Skip memories with no useful content
        if (!mem.summary && !mem.content) {
          totalSkipped++;
          continue;
        }

        const input = buildEmbeddingInput(mem.summary, mem.content);
        const embedding = await getEmbedding(input);

        // Convert to pgvector format
        const vectorStr = `[${embedding.join(',')}]`;

        await client.query(`
          UPDATE "Memory"
          SET embedding = $1::vector, "updatedAt" = NOW()
          WHERE id = $2
        `, [vectorStr, mem.id]);

        totalUpdated++;

        if (totalUpdated % 10 === 0) {
          process.stdout.write(`  ✅ ${totalUpdated} updated, ${totalFailed} failed, ${totalSkipped} skipped\n`);
        }

        // Small delay to avoid overwhelming Ollama
        await new Promise(r => setTimeout(r, 50));

      } catch (e) {
        totalFailed++;
        console.error(`  ❌ Failed ${mem.id} (${mem.summary?.substring(0, 40)}...): ${e.message}`);
      }
    }

    // Check remaining count
    const { rows: remaining } = await client.query(`
      SELECT COUNT(*) as count FROM "Memory" WHERE embedding IS NULL
    `);

    if (parseInt(remaining[0].count) === 0) {
      console.log('\n✅ All memories now have embeddings!');
      break;
    }
  }

  // ── Generate content fingerprints for source field ─────────────────────────
  console.log('\n--- Generating content fingerprints for null source fields ---');

  const { rows: nullSource } = await client.query(`
    SELECT id, summary, content FROM "Memory"
    WHERE "source" IS NULL
    LIMIT 500
  `);

  console.log(`Found ${nullSource.length} memories with null source`);

  let fpUpdated = 0;
  for (const mem of nullSource) {
    if (!mem.summary && !mem.content) continue;
    const fp = makeContentFingerprint(mem.summary || '', mem.content || '');
    try {
      await client.query(`UPDATE "Memory" SET "source" = $1 WHERE id = $2`, [`fp://${fp.substring(0, 16)}`, mem.id]);
      fpUpdated++;
    } catch (e) {
      // ignore
    }
  }
  console.log(`Updated ${fpUpdated} source fingerprints`);

  // ── Final stats ───────────────────────────────────────────────────────────
  console.log('\n--- Final Stats ---');

  const { rows: finalStats } = await client.query(`
    SELECT
      COUNT(*) as total,
      COUNT(embedding) as has_embedding,
      COUNT("source") as has_source
    FROM "Memory"
  `);
  console.log(`Total: ${finalStats[0].total} | With embedding: ${finalStats[0].has_embedding} | With source: ${finalStats[0].has_source}`);

  // Similarity sample test
  const { rows: withEmb } = await client.query(`
    SELECT summary, embedding FROM "Memory" WHERE embedding IS NOT NULL LIMIT 2
  `);
  if (withEmb.length >= 2) {
    const vec1 = withEmb[0].embedding;
    const vec2 = withEmb[1].embedding;
    const { rows: simTest } = await client.query(`
      SELECT
        $1::vector <=> $2::vector as distance,
        1 - ($1::vector <=> $2::vector) as similarity
    `, [vec1, vec2]);
    console.log(`\nSimilarity test between first 2 embeddings: ${parseFloat(simTest[0].similarity).toFixed(4)}`);
  }

  console.log(`\nEmbedding generation: ${totalUpdated} updated, ${totalFailed} failed, ${totalSkipped} skipped`);
  await client.end();
}

main().catch(e => { console.error(e); process.exit(1); });
