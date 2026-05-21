/**
 * Regenerate null fingerprint values for all ContextDocument entries.
 * Uses node-postgres directly to avoid Prisma pool issues.
 *
 * Run: node scripts/regenerate-doc-fingerprints.mjs
 */
import { createHash } from 'crypto';
import { Client } from 'pg';

const client = new Client({
  host: 'db.REDACTED.supabase.co',
  port: 5432,
  user: 'postgres',
  password: 'REDACTED',
  database: 'postgres',
});

function makeDocFingerprint(doc) {
  const payload = JSON.stringify({
    id: doc.id,
    workflowId: doc.workflowId,
    sourceType: doc.sourceType,
    sourceRef: doc.sourceRef,
    title: doc.title,
    contentHash: createHash('sha256').update(doc.content || '').digest('hex'),
    metadata: doc.metadata,
  });
  return createHash('sha256').update(payload).digest('hex');
}

async function main() {
  await client.connect();
  console.log('Connected to PostgreSQL');

  const { rows: docs } = await client.query(`
    SELECT id, "workflowId", "sourceType", "sourceRef", title, content, metadata
    FROM "ContextDocument"
    WHERE fingerprint IS NULL OR fingerprint = ''
  `);

  console.log('Found', docs.length, 'ContextDocuments with null fingerprints');

  let updated = 0;
  let skipped = 0;

  for (const doc of docs) {
    try {
      const fp = makeDocFingerprint({
        id: doc.id,
        workflowId: doc.workflowId,
        sourceType: doc.sourceType,
        sourceRef: doc.sourceRef,
        title: doc.title,
        content: doc.content || '',
        metadata: doc.metadata,
      });

      await client.query(`
        UPDATE "ContextDocument" SET fingerprint = $1 WHERE id = $2
      `, [fp, doc.id]);

      updated++;
      if (updated % 25 === 0) console.log('  ...', updated, 'updated');
    } catch (e) {
      skipped++;
      console.error('Failed for doc', doc.id, ':', e.message);
    }
  }

  console.log('Updated:', updated, '| Skipped:', skipped);

  // Verify
  const { rows: remaining } = await client.query(`
    SELECT COUNT(*) as count FROM "ContextDocument"
    WHERE fingerprint IS NULL OR fingerprint = ''
  `);
  console.log('Remaining null fingerprints:', remaining[0].count);

  await client.end();
}

main().catch(e => { console.error(e); process.exit(1); });
