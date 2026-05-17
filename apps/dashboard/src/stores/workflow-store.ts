// ============================================================
// Workflow Store — workflows list, active workflow, tasks
// ============================================================

import { create } from 'zustand';
import { workflowApi } from '@/lib/api-client';
import type { Workflow, Task } from '@/lib/types';

interface WorkflowState {
  workflows: Workflow[];
  activeWorkflow: Workflow | null;
  selectedWorkflow: Workflow | null;
  tasks: Task[];
  isLoading: boolean;
  error: string | null;
  fetchWorkflows: () => Promise<void>;
  fetchWorkflow: (id: string) => Promise<void>;
  fetchActive: () => Promise<void>;
  createWorkflow: (name: string, description: string, metadata?: Record<string, unknown>) => Promise<Workflow>;
  executeWorkflow: (id: string) => Promise<void>;
  fetchTasks: (workflowId: string) => Promise<void>;
  addTask: (workflowId: string, task: { name: string; agent: string; skill: string; dependencies?: string[]; input?: unknown }) => Promise<Task>;
  updateTaskState: (workflowId: string, taskId: string, state: string) => void;
  clearError: () => void;
}

export const useWorkflowStore = create<WorkflowState>((set, get) => ({
  workflows: [],
  activeWorkflow: null,
  selectedWorkflow: null,
  tasks: [],
  isLoading: false,
  error: null,

  fetchWorkflows: async () => {
    set({ isLoading: true });
    try {
      const result = await workflowApi.list();
      set({ workflows: result.workflows, isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch workflows', isLoading: false });
    }
  },

  fetchWorkflow: async (id: string) => {
    set({ isLoading: true });
    try {
      const result = await workflowApi.get(id);
      set({ selectedWorkflow: result.workflow, tasks: result.workflow.tasks || [], isLoading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch workflow', isLoading: false });
    }
  },

  fetchActive: async () => {
    try {
      const result = await workflowApi.getActive();
      set({ activeWorkflow: result.workflow });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch active workflow' });
    }
  },

  createWorkflow: async (name: string, description: string, metadata?: Record<string, unknown>) => {
    set({ isLoading: true });
    try {
      const result = await workflowApi.create(name, description, metadata);
      const current = get().workflows;
      set({ workflows: [...current, result.workflow], isLoading: false });
      return result.workflow;
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to create workflow', isLoading: false });
      throw err;
    }
  },

  executeWorkflow: async (id: string) => {
    try {
      const result = await workflowApi.execute(id);
      const workflows = get().workflows.map((w) =>
        w.id === id ? result.workflow : w,
      );
      set({ workflows, selectedWorkflow: result.workflow });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to execute workflow' });
    }
  },

  fetchTasks: async (workflowId: string) => {
    try {
      const result = await workflowApi.getTasks(workflowId);
      set({ tasks: result.tasks });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch tasks' });
    }
  },

  addTask: async (workflowId: string, task) => {
    const result = await workflowApi.addTask(workflowId, task);
    set((state) => ({ tasks: [...state.tasks, result.task] }));
    return result.task;
  },

  updateTaskState: (workflowId: string, taskId: string, state: string) => {
    set((s) => ({
      tasks: s.tasks.map((t) => t.id === taskId ? { ...t, state } : t),
      selectedWorkflow: s.selectedWorkflow
        ? {
            ...s.selectedWorkflow,
            tasks: s.selectedWorkflow.tasks.map((t) =>
              t.id === taskId ? { ...t, state } : t,
            ),
          }
        : null,
    }));
  },

  clearError: () => set({ error: null }),
}));
