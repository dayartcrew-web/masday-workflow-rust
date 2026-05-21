import { createHash } from 'crypto';
import prismaPkg from '@prisma/client';
const { PrismaClient } = prismaPkg;

const prisma = new PrismaClient();

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
  const tasks = await prisma.task.findMany({
    where: { OR: [{ contextFingerprint: null }, { contextFingerprint: '' }] },
    include: { plan: true },
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
        acceptanceCriteria: Array.isArray(task.acceptanceCriteria) ? task.acceptanceCriteria : [],
        requiredContext: Array.isArray(task.requiredContext) ? task.requiredContext : [],
        documentIds: docIds.map(d => d.id),
        memoryIds: memIds.map(m => m.id),
      });

      await prisma.task.update({
        where: { id: task.id },
        data: { contextFingerprint: fp },
      });
      updated++;
    } catch (e) {
      skipped++;
      console.error('Failed for task', task.id, ':', e.message);
    }
  }

  console.log('Updated:', updated, '| Skipped:', skipped);
  await prisma.$disconnect();
}

main().catch(e => { console.error(e); process.exit(1); });
