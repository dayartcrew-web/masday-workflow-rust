/**
 * Regenerate null contextFingerprint values for all tasks.
 * Uses node-postgres directly to avoid Prisma pool issues.
 *
 * Run: node scripts/regenerate-fingerprints-pg.mjs
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

function makeFingerprint(input) {
  const sortable = {
    acceptanceCriteria: [...(input.acceptanceCriteria || [])].sort(),
    requiredContext: [...(input.requiredContext || [])].sort(),
    documentIds: [...(input.documentIds || [])].sort(),
    memoryIds: [...(input.memoryIds || [])].sort(),
  };
  const payload = JSON.stringify({
    workflowId: input.workflowId,
    planId: input.planId,
    taskId: input.taskId,
    ...sortable,
  });
  return createHash('sha256').update(payload).digest('hex');
}

async function main() {
  await client.connect();
  console.log('Connected to PostgreSQL');

  const { rows: tasks } = await client.query(`
    SELECT id, "workflowId", "planId", "acceptanceCriteria", "requiredContext"
    FROM "Task"
    WHERE "contextFingerprint" IS NULL OR "contextFingerprint" = ''
  `);

  console.log('Found', tasks.length, 'tasks with null fingerprints');

  let updated = 0;
  let skipped = 0;

  for (const task of tasks) {
    try {
      // Get memory IDs for this workflow
      const { rows: memRows } = await client.query(`
        SELECT id FROM "Memory"
        WHERE "workflowId" = $1 OR "taskId" = $2
        ORDER BY "importanceScore" DESC
        LIMIT 20
      `, [task.workflowId, task.id]);

      // Get doc IDs for this workflow
      const { rows: docRows } = await client.query(`
        SELECT id FROM "ContextDocument"
        WHERE "workflowId" = $1
        ORDER BY "createdAt" DESC
        LIMIT 10
      `, [task.workflowId]);

      const fp = makeFingerprint({
        workflowId: task.workflowId,
        planId: task.planId,
        taskId: task.id,
        acceptanceCriteria: Array.isArray(task.acceptanceCriteria) ? task.acceptanceCriteria : [],
        requiredContext: Array.isArray(task.requiredContext) ? task.requiredContext : [],
        documentIds: docRows.map(r => r.id),
        memoryIds: memRows.map(r => r.id),
      });

      await client.query(`
        UPDATE "Task" SET "contextFingerprint" = $1 WHERE id = $2
      `, [fp, task.id]);

      updated++;
      if (updated % 50 === 0) console.log('  ...', updated, 'updated');
    } catch (e) {
      skipped++;
      console.error('Failed for task', task.id, ':', e.message);
    }
  }

  console.log('Updated:', updated, '| Skipped:', skipped);

  // Verify
  const { rows: remaining } = await client.query(`
    SELECT COUNT(*) as count FROM "Task"
    WHERE "contextFingerprint" IS NULL OR "contextFingerprint" = ''
  `);
  console.log('Remaining null fingerprints:', remaining[0].count);

  await client.end();
}

main().catch(e => { console.error(e); process.exit(1); });
