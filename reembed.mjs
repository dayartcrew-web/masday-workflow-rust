/**
 * Re-embed memories with null embedding using Ollama nomic-embed-text.
 * Patched for snake_case PostgreSQL schema.
 * Usage: node reembed.mjs --local --batch-size 50
 */
import { createHash } from 'crypto';
import pg from 'pg';
const { Client } = pg;

const IS_LOCAL = process.argv.includes('--local');
const BATCH_SIZE = parseInt(process.argv.find(a => a.startsWith('--batch-size='))?.split('=')[1]) || 50;

const DB_CONFIG = IS_LOCAL
  ? { host: 'localhost', port: 54341, user: 'postgres', password: 'postgres', database: 'masday_workflow' }
  : { host: 'db.gvjevnfmxiqpikvxiunc.supabase.co', port: 5432, user: 'postgres', password: 'pU335NIco29IwqsQ', database: 'postgres' };

const OLLAMA_URL = process.env.OLLAMA_URL || 'http://localhost:11434';
const OLLAMA_MODEL = process.env.OLLAMA_MODEL || 'nomic-embed-text:latest';
const EXPECTED_DIMS = 768;

async function getEmbedding(text) {
  const truncated = text.length > 2000 ? text.substring(0, 2000) : text;
  try {
    const res = await fetch(`${OLLAMA_URL}/api/embed`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ model: OLLAMA_MODEL, input: truncated }),
    });
    if (res.ok) {
      const data = await res.json();
      if (data.embeddings?.[0]?.length === EXPECTED_DIMS) return data.embeddings[0];
    }
  } catch {}
  const res = await fetch(`${OLLAMA_URL}/api/embeddings`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model: OLLAMA_MODEL, prompt: truncated }),
  });
  if (!res.ok) throw new Error(`Ollama returned ${res.status}: ${await res.text()}`);
  const data = await res.json();
  if (!data.embedding || data.embedding.length !== EXPECTED_DIMS) throw new Error(`Unexpected dims: ${data.embedding?.length}`);
  return data.embedding;
}

function buildEmbeddingInput(summary, content) {
  return `${summary}\n${(content || '').substring(0, 500)}`.trim();
}

async function main() {
  const client = new Client(DB_CONFIG);
  await client.connect();
  console.log(`Connected to ${IS_LOCAL ? 'local' : 'Supabase'} PostgreSQL`);

  const { rows: extRows } = await client.query(`SELECT extname, extversion FROM pg_extension WHERE extname = 'vector'`);
  if (extRows.length === 0) { console.error('ERROR: pgvector not installed'); process.exit(1); }
  console.log(`pgvector v${extRows[0].extversion} available`);

  const { rows: stats } = await client.query(`SELECT COUNT(*) as total, COUNT(embedding) as has_embedding, COUNT(*) - COUNT(embedding) as needs_embedding FROM memories`);
  console.log(`Memory stats: ${stats[0].total} total, ${stats[0].has_embedding} with embedding, ${stats[0].needs_embedding} need embedding`);

  if (parseInt(stats[0].needs_embedding) === 0) { console.log('All memories already have embeddings.'); await client.end(); return; }

  // Check Ollama
  const ollamaRes = await fetch(`${OLLAMA_URL}/api/tags`);
  const ollamaData = await ollamaRes.json();
  const models = ollamaData.models?.map(m => m.name) || [];
  console.log(`Ollama models: ${models.join(', ')}`);

  let totalUpdated = 0, totalFailed = 0, totalSkipped = 0;

  while (true) {
    const { rows: memories } = await client.query(`SELECT id, summary, content FROM memories WHERE embedding IS NULL ORDER BY created_at ASC LIMIT $1`, [BATCH_SIZE]);
    if (memories.length === 0) break;
    console.log(`\nBatch: ${memories.length} memories (${totalUpdated} done so far)`);

    for (const mem of memories) {
      try {
        if (!mem.summary && !mem.content) { totalSkipped++; continue; }
        const input = buildEmbeddingInput(mem.summary, mem.content);
        const embedding = await getEmbedding(input);
        const vectorStr = `[${embedding.join(',')}]`;
        await client.query(`UPDATE memories SET embedding = $1::vector, updated_at = NOW() WHERE id = $2`, [vectorStr, mem.id]);
        totalUpdated++;
        if (totalUpdated % 50 === 0) process.stdout.write(`  ✅ ${totalUpdated} updated, ${totalFailed} failed\n`);
        await new Promise(r => setTimeout(r, 30));
      } catch (e) {
        totalFailed++;
        if (totalFailed <= 5) console.error(`  ❌ ${mem.id}: ${e.message}`);
      }
    }

    const { rows: rem } = await client.query(`SELECT COUNT(*) as c FROM memories WHERE embedding IS NULL`);
    if (parseInt(rem[0].c) === 0) { console.log('\n✅ All memories embedded!'); break; }
  }

  const { rows: fin } = await client.query(`SELECT COUNT(*) as total, COUNT(embedding) as has_embedding FROM memories`);
  console.log(`\nFinal: ${fin[0].total} total, ${fin[0].has_embedding} with embedding`);
  console.log(`Done: ${totalUpdated} updated, ${totalFailed} failed, ${totalSkipped} skipped`);
  await client.end();
}

main().catch(e => { console.error(e); process.exit(1); });
