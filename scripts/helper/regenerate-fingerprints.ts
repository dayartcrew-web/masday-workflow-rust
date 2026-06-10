/**
 * Regenerate null contextFingerprint values for all tasks.
 * Uses the db package's Drizzle instance.
 *
 * Run: npx tsx scripts/regenerate-fingerprints.ts
 */
import { createHash } from 'crypto';
import { db, disconnectDb } from '../packages/db/src/index.js';
import { tasks, memories, contextDocuments } from '../packages/db/src/schema.js';
import { isNull, or, eq, desc } from 'drizzle-orm';
import { createLogger } from '../packages/core/src/logger.js';

const logger = createLogger('regenerate-fingerprints');

function makeFingerprint(input: {
  workflowId: string;
  planId: string;
  taskId: string;
  acceptanceCriteria: string[];
  requiredContext: string[];
  documentIds: string[];
  memoryIds: string[];
}): string {
  const sortable = {
    acceptanceCriteria: [...input.acceptanceCriteria].sort(),
    requiredContext: [...input.requiredContext].sort(),
    documentIds: [...input.documentIds].sort(),
    memoryIds: [...input.memoryIds].sort(),
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
  const nullTasks = await db.select().from(tasks)
    .where(or(isNull(tasks.contextFingerprint), eq(tasks.contextFingerprint, '')));

  logger.info('Found ' + nullTasks.length + ' tasks with null fingerprints');

  let updated = 0;
  let skipped = 0;

  for (const task of nullTasks) {
    try {
      const memRows = await db.select({ id: memories.id }).from(memories)
        .where(or(eq(memories.workflowId, task.workflowId), eq(memories.taskId, task.id)))
        .orderBy(desc(memories.importanceScore))
        .limit(20);

      const docRows = await db.select({ id: contextDocuments.id }).from(contextDocuments)
        .where(eq(contextDocuments.workflowId, task.workflowId))
        .orderBy(desc(contextDocuments.createdAt))
        .limit(10);

      const fp = makeFingerprint({
        workflowId: task.workflowId,
        planId: task.planId,
        taskId: task.id,
        acceptanceCriteria: Array.isArray(task.acceptanceCriteria) ? (task.acceptanceCriteria as string[]) : [],
        requiredContext: Array.isArray(task.requiredContext) ? (task.requiredContext as string[]) : [],
        documentIds: docRows.map(d => d.id),
        memoryIds: memRows.map(m => m.id),
      });

      await db.update(tasks)
        .set({ contextFingerprint: fp })
        .where(eq(tasks.id, task.id));
      updated++;
    } catch (e: unknown) {
      skipped++;
      logger.error('Failed for task ' + task.id + ': ' + (e instanceof Error ? e.message : String(e)));
    }
  }

  logger.info('Updated: ' + updated + ' | Skipped: ' + skipped);

  const remaining = await db.select().from(tasks)
    .where(or(isNull(tasks.contextFingerprint), eq(tasks.contextFingerprint, '')));
  logger.info('Remaining null fingerprints: ' + remaining.length);

  await disconnectDb();
}

main().catch(e => { logger.error(String(e)); process.exit(1); });
