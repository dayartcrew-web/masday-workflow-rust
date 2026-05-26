import { pgTable, uuid, text, integer, doublePrecision, boolean, jsonb, timestamp, index, uniqueIndex } from "drizzle-orm/pg-core";
import { vector } from "./vector.js";

export const workflows = pgTable("Workflow", {
  id: uuid("id").primaryKey().defaultRandom(),
  name: text("name").notNull(),
  status: text("status").notNull(),
  projectPath: text("projectPath"),
  currentPlanId: text("currentPlanId"),
  currentTaskId: text("currentTaskId"),
  metadata: jsonb("metadata").default({}),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
}, (t) => [
  index("Workflow_currentPlanId_idx").on(t.currentPlanId),
  index("Workflow_currentTaskId_idx").on(t.currentTaskId),
  index("Workflow_projectPath_idx").on(t.projectPath),
]);

export const plans = pgTable("Plan", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  version: integer("version").notNull(),
  status: text("status").notNull(),
  summary: text("summary").notNull(),
  content: jsonb("content").notNull(),
  createdByAgent: text("createdByAgent").notNull(),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("Plan_workflowId_idx").on(t.workflowId),
]);

export const tasks = pgTable("Task", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  planId: text("planId").notNull(),
  title: text("title").notNull(),
  status: text("status").notNull(),
  priority: text("priority"),
  ownerAgent: text("ownerAgent"),
  acceptanceCriteria: jsonb("acceptanceCriteria").default([]),
  requiredContext: jsonb("requiredContext").default([]),
  verificationSteps: jsonb("verificationSteps").default([]),
  contextFingerprint: text("contextFingerprint"),
  progressPercent: integer("progressPercent").default(0),
  requiresTdd: boolean("requiresTdd").default(false),
  testEvidence: jsonb("testEvidence").default({}),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
}, (t) => [
  index("Task_workflowId_idx").on(t.workflowId),
  index("Task_planId_idx").on(t.planId),
]);

export const taskProgressLogs = pgTable("TaskProgressLog", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  taskId: text("taskId").notNull(),
  agentName: text("agentName").notNull(),
  statusBefore: text("statusBefore"),
  statusAfter: text("statusAfter"),
  progressNote: text("progressNote").notNull(),
  evidence: jsonb("evidence").default([]),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("TaskProgressLog_workflowId_idx").on(t.workflowId),
  index("TaskProgressLog_taskId_idx").on(t.taskId),
]);

export const reviewDecisions = pgTable("ReviewDecision", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  taskId: text("taskId").notNull(),
  reviewerAgent: text("reviewerAgent").notNull(),
  decision: text("decision").notNull(),
  notes: text("notes").notNull(),
  gaps: jsonb("gaps").default([]),
  testsVerified: boolean("testsVerified").default(false),
  testSummary: jsonb("testSummary").default({}),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("ReviewDecision_workflowId_idx").on(t.workflowId),
  index("ReviewDecision_taskId_idx").on(t.taskId),
]);

export const sessionStates = pgTable("SessionState", {
  id: uuid("id").primaryKey().defaultRandom(),
  sessionKey: text("sessionKey").notNull().unique(),
  workflowId: text("workflowId"),
  planId: text("planId"),
  taskId: text("taskId"),
  workflowLoaded: boolean("workflowLoaded").default(false),
  planLoaded: boolean("planLoaded").default(false),
  taskLoaded: boolean("taskLoaded").default(false),
  contextLoaded: boolean("contextLoaded").default(false),
  reviewApproved: boolean("reviewApproved").default(false),
  contextFingerprint: text("contextFingerprint"),
  executionMode: text("executionMode"),
  activeBranchIds: jsonb("activeBranchIds").default([]),
  synthesisReady: boolean("synthesisReady").default(false),
  verificationReady: boolean("verificationReady").default(false),
  lastCommand: text("lastCommand"),
  metadata: jsonb("metadata").default({}),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("SessionState_workflowId_idx").on(t.workflowId),
]);

export const parallelBranches = pgTable("ParallelBranch", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  taskId: text("taskId").notNull(),
  branchKey: text("branchKey").notNull(),
  role: text("role").notNull(),
  status: text("status").notNull(),
  input: jsonb("input").notNull(),
  output: jsonb("output"),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
}, (t) => [
  index("ParallelBranch_workflowId_idx").on(t.workflowId),
  index("ParallelBranch_taskId_idx").on(t.taskId),
]);

export const retrievalLogs = pgTable("RetrievalLog", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId"),
  taskId: text("taskId"),
  agentName: text("agentName").notNull(),
  query: text("query").notNull(),
  source: text("source").notNull(),
  results: jsonb("results").default([]),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("RetrievalLog_workflowId_idx").on(t.workflowId),
]);

export const memories = pgTable("Memory", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId"),
  taskId: text("taskId"),
  memoryType: text("memoryType").notNull(),
  summary: text("summary").notNull(),
  content: text("content").notNull(),
  importanceScore: doublePrecision("importanceScore").default(0.5),
  createdByAgent: text("createdByAgent").notNull(),
  tags: text("tags").array().notNull(),
  source: text("source"),
  embedding: vector(768)("embedding"),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
  accessedAt: timestamp("accessedAt"),
  accessCount: integer("accessCount").default(0),
  version: integer("version").default(1),
}, (t) => [
  index("Memory_workflowId_idx").on(t.workflowId),
  index("Memory_memoryType_idx").on(t.memoryType),
  index("Memory_importanceScore_idx").on(t.importanceScore),
]);

export const contextDocuments = pgTable("ContextDocument", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId"),
  sourceType: text("sourceType").notNull(),
  sourceRef: text("sourceRef"),
  title: text("title"),
  content: text("content").notNull(),
  metadata: jsonb("metadata").default({}),
  fingerprint: text("fingerprint"),
  embedding: vector(768)("embedding"),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull().$onUpdate(() => new Date()),
}, (t) => [
  index("ContextDocument_workflowId_idx").on(t.workflowId),
]);

export const graphNodes = pgTable("GraphNode", {
  id: uuid("id").primaryKey().defaultRandom(),
  nodeType: text("nodeType").notNull(),
  name: text("name").notNull(),
  properties: jsonb("properties"),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("GraphNode_nodeType_idx").on(t.nodeType),
  index("GraphNode_name_idx").on(t.name),
]);

export const graphEdges = pgTable("GraphEdge", {
  id: uuid("id").primaryKey().defaultRandom(),
  sourceNodeId: text("sourceNodeId").notNull(),
  targetNodeId: text("targetNodeId").notNull(),
  relationType: text("relationType").notNull(),
  weight: doublePrecision("weight").default(1.0),
  bidirectional: boolean("bidirectional").default(false),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("GraphEdge_sourceNodeId_idx").on(t.sourceNodeId),
  index("GraphEdge_targetNodeId_idx").on(t.targetNodeId),
  index("GraphEdge_relationType_idx").on(t.relationType),
]);

export const episodicMemories = pgTable("EpisodicMemory", {
  id: uuid("id").primaryKey().defaultRandom(),
  sessionId: text("sessionId").notNull(),
  role: text("role").notNull(),
  content: text("content").notNull(),
  sequenceOrder: integer("sequenceOrder").notNull(),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("EpisodicMemory_sessionId_sequenceOrder_idx").on(t.sessionId, t.sequenceOrder),
]);

export const workflowReminders = pgTable("WorkflowReminder", {
  id: uuid("id").primaryKey().defaultRandom(),
  workflowId: text("workflowId").notNull(),
  taskId: text("taskId"),
  type: text("type").notNull(),
  severity: text("severity").notNull(),
  message: text("message").notNull(),
  acknowledged: boolean("acknowledged").default(false),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("WorkflowReminder_workflowId_idx").on(t.workflowId),
  index("WorkflowReminder_acknowledged_idx").on(t.acknowledged),
  index("WorkflowReminder_severity_idx").on(t.severity),
]);

export const llmProviderConfigs = pgTable("LlmProviderConfig", {
  id: uuid("id").primaryKey().defaultRandom(),
  providerName: text("providerName").notNull().unique(),
  baseUrl: text("baseUrl").notNull(),
  apiKeyEnvVar: text("apiKeyEnvVar").notNull(),
  models: jsonb("models").notNull(),
  isDefault: boolean("isDefault").default(false),
  priority: integer("priority").default(0),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
  updatedAt: timestamp("updatedAt").defaultNow().notNull(),
});

export const tokenUsages = pgTable("TokenUsage", {
  id: uuid("id").primaryKey().defaultRandom(),
  source: text("source").notNull(),
  route: text("route").notNull(),
  model: text("model"),
  promptTokens: integer("promptTokens").default(0),
  completionTokens: integer("completionTokens").default(0),
  totalTokens: integer("totalTokens").default(0),
  latencyMs: integer("latencyMs").default(0),
  metadata: jsonb("metadata").default({}),
  createdAt: timestamp("createdAt").defaultNow().notNull(),
}, (t) => [
  index("TokenUsage_source_idx").on(t.source),
  index("TokenUsage_route_idx").on(t.route),
  index("TokenUsage_createdAt_idx").on(t.createdAt),
  index("TokenUsage_model_idx").on(t.model),
]);
