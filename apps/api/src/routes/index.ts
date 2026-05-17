// ============================================================
// Route index — aggregates all route modules
// ============================================================

export { authRoutes } from './auth';
export { createWorkflowRoutes } from './workflows';
export { createMemoryRoutes } from './memory';
export type { MemoryServiceProvider } from './memory';
export { createSearchRoutes } from './search';
export type { SearchServiceProvider } from './search';
export { createPolicyRoutes } from './policy';
export type { PolicyServiceProvider } from './policy';
export { createCapabilityRoutes } from './capability';
export type { CapabilityServiceProvider } from './capability';
export { createChatRoutes } from './chat';
export type { ChatServiceProvider } from './chat';
export { createProviderRoutes } from './providers';
export type { ProviderServiceProvider } from './providers';
export { createMonitoringRoutes } from './monitoring';
export type { MonitoringServiceProvider } from './monitoring';
