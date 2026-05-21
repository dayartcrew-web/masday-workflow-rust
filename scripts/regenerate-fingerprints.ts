/**
 * Regenerate null contextFingerprint values for all tasks.
 * Uses the db package's prisma instance which is already configured.
 *
 * Run: npx tsx scripts/regenerate-fingerprints.ts
 */
import { createHash } from 'crypto';
import { prisma } from '../packages/db/src/index.js';

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
  const tasks = await prisma.task.findMany({
    where: { OR: [{ contextFingerprint: null }, { contextFingerprint: '' }] },
  });

  console.log('Found', tasks.length, 'tasks with null fingerprints');

  let updated = 0;
  let skipped = 0;

  for (const task of tasks) {
    try {
      const memIds = await prisma.memory.findMany({
        where: { OR: [{ workflowId: task.workflowId }, { taskId: task.id }] },
        select: { id: true },
        take: 20,
        orderBy: { importanceScore: 'desc' },
      });
      const docIds = await prisma.contextDocument.findMany({
        where: { workflowId: task.workflowId },
        select: { id: true },
        take: 10,
        orderBy: { createdAt: 'desc' },
      });

      const fp = makeFingerprint({
        workflowId: task.workflowId,
        planId: task.planId,
        taskId: task.id,
        acceptanceCriteria: Array.isArray(task.acceptanceCriteria) ? (task.acceptanceCriteria as string[]) : [],
        requiredContext: Array.isArray(task.requiredContext) ? (task.requiredContext as string[]) : [],
        documentIds: docIds.map((d: { id: string }) => d.id),
        memoryIds: memIds.map((m: { id: string }) => m.id),
      });

      await prisma.task.update({
        where: { id: task.id },
        data: { contextFingerprint: fp },
      });
      updated++;
    } catch (e: unknown) {
      skipped++;
      console.error('Failed for task', task.id, ':', e instanceof Error ? e.message : String(e));
    }
  }

  console.log('Updated:', updated, '| Skipped:', skipped);

  // Verify
  const remaining = await prisma.task.count({
    where: { OR: [{ contextFingerprint: null }, { contextFingerprint: '' }] },
  });
  console.log('Remaining null fingerprints:', remaining);

  await prisma.$disconnect();
}

main().catch(e => { console.error(e); process.exit(1); });
