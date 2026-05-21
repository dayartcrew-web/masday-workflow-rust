import { relations } from "drizzle-orm";
import { workflows, plans, tasks, taskProgressLogs, reviewDecisions, memories, contextDocuments } from "./schema.js";

export const workflowRelations = relations(workflows, ({ many }) => ({
  plans: many(plans),
  tasks: many(tasks),
  contexts: many(contextDocuments),
  memories: many(memories),
  reviews: many(reviewDecisions),
  progressLogs: many(taskProgressLogs),
}));

export const planRelations = relations(plans, ({ one, many }) => ({
  workflow: one(workflows, { fields: [plans.workflowId], references: [workflows.id] }),
  tasks: many(tasks),
}));

export const taskRelations = relations(tasks, ({ one, many }) => ({
  workflow: one(workflows, { fields: [tasks.workflowId], references: [workflows.id] }),
  plan: one(plans, { fields: [tasks.planId], references: [plans.id] }),
  progressLogs: many(taskProgressLogs),
  reviews: many(reviewDecisions),
}));

export const taskProgressLogRelations = relations(taskProgressLogs, ({ one }) => ({
  workflow: one(workflows, { fields: [taskProgressLogs.workflowId], references: [workflows.id] }),
  task: one(tasks, { fields: [taskProgressLogs.taskId], references: [tasks.id] }),
}));

export const reviewDecisionRelations = relations(reviewDecisions, ({ one }) => ({
  workflow: one(workflows, { fields: [reviewDecisions.workflowId], references: [workflows.id] }),
  task: one(tasks, { fields: [reviewDecisions.taskId], references: [tasks.id] }),
}));

export const memoryRelations = relations(memories, ({ one }) => ({
  workflow: one(workflows, { fields: [memories.workflowId], references: [workflows.id] }),
}));

export const contextDocumentRelations = relations(contextDocuments, ({ one }) => ({
  workflow: one(workflows, { fields: [contextDocuments.workflowId], references: [workflows.id] }),
}));
