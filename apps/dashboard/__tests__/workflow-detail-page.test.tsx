import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WorkflowDetailPage from '@/app/workflows/[id]/page';
import type { Task, Workflow } from '@/lib/types';

const push = vi.fn();

vi.mock('next/navigation', () => ({
  useParams: () => ({ id: 'wf-1' }),
  useRouter: () => ({ push }),
}));

vi.mock('@/components/app-shell', () => ({
  AppShell: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/workflow-dag', () => ({
  WorkflowDag: () => <div data-testid="workflow-dag" />,
}));

const workflowStoreState: {
  selectedWorkflow: Workflow | null;
  isLoading: boolean;
  fetchWorkflow: ReturnType<typeof vi.fn>;
  executeWorkflow: ReturnType<typeof vi.fn>;
  updateTaskState: ReturnType<typeof vi.fn>;
} = {
  selectedWorkflow: null,
  isLoading: false,
  fetchWorkflow: vi.fn(),
  executeWorkflow: vi.fn(),
  updateTaskState: vi.fn(),
};

const websocketStoreState = {
  latestEvent: null as { type: string; data: unknown } | null,
};

vi.mock('@/stores/workflow-store', () => ({
  useWorkflowStore: (selector: (state: typeof workflowStoreState) => unknown) => selector(workflowStoreState),
}));

vi.mock('@/stores/websocket-store', () => ({
  useWebSocketStore: (selector: (state: typeof websocketStoreState) => unknown) => selector(websocketStoreState),
}));

function createTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    name: 'Plan merge',
    agent: 'planner',
    skill: 'workflow',
    state: 'pending',
    dependencies: [],
    ...overrides,
  };
}

function createWorkflow(task: Task): Workflow {
  return {
    id: 'wf-1',
    name: 'Merge workflow',
    description: 'Test workflow',
    state: 'running',
    tasks: [task],
    metadata: {},
    createdAt: '2026-05-16T00:00:00.000Z',
    updatedAt: '2026-05-16T00:00:00.000Z',
  };
}

describe('WorkflowDetailPage', () => {
  beforeEach(() => {
    push.mockReset();
    workflowStoreState.fetchWorkflow.mockReset();
    workflowStoreState.executeWorkflow.mockReset();
    workflowStoreState.updateTaskState.mockReset();
    websocketStoreState.latestEvent = null;
  });

  it('keeps selected task details in sync with the latest workflow task state', () => {
    workflowStoreState.selectedWorkflow = createWorkflow(createTask({ state: 'pending' }));

    const { rerender } = render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByText('Plan merge')[0]);
    expect(screen.getByText((_, element) => element?.textContent === 'State: pending')).toBeInTheDocument();

    workflowStoreState.selectedWorkflow = createWorkflow(createTask({ state: 'running' }));
    rerender(<WorkflowDetailPage />);

    expect(screen.getByText((_, element) => element?.textContent === 'State: running')).toBeInTheDocument();
    expect(screen.queryByText((_, element) => element?.textContent === 'State: pending')).not.toBeInTheDocument();
  });

  it('ignores task.* websocket events for other workflows when workflowId is present', () => {
    workflowStoreState.selectedWorkflow = createWorkflow(createTask({ state: 'pending' }));

    const { rerender } = render(<WorkflowDetailPage />);

    websocketStoreState.latestEvent = {
      type: 'task.updated',
      data: { workflowId: 'wf-other', taskId: 'task-1', state: 'running' },
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).not.toHaveBeenCalled();
  });

  it('applies task.* websocket events without workflowId only when the task belongs to the selected workflow', () => {
    workflowStoreState.selectedWorkflow = createWorkflow(createTask({ id: 'task-1', state: 'pending' }));

    const { rerender } = render(<WorkflowDetailPage />);

    websocketStoreState.latestEvent = {
      type: 'task.updated',
      data: { taskId: 'task-1', state: 'running' },
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).toHaveBeenCalledWith('wf-1', 'task-1', 'running');

    workflowStoreState.updateTaskState.mockClear();
    websocketStoreState.latestEvent = {
      type: 'task.updated',
      data: { taskId: 'task-other', state: 'running' },
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).not.toHaveBeenCalled();
  });
});
