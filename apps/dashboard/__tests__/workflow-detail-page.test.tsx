import React from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WorkflowDetailPage from '@/app/workflows/[id]/page';
import type { Task, Workflow } from '@/lib/types';

const push = vi.fn();
const replace = vi.fn();

vi.mock('next/navigation', () => ({
  useParams: () => ({ id: 'wf-1' }),
  useRouter: () => ({ push, replace }),
}));

vi.mock('@/components/app-shell', () => ({
  AppShell: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/workflow-dag', () => ({
  WorkflowDag: ({ tasks, onTaskClick }: { tasks: Task[]; onTaskClick?: (task: Task) => void }) => (
    <div data-testid="workflow-dag">
      {tasks.map((t) => (
        <button key={t.id} onClick={() => onTaskClick?.(t)} data-testid={`dag-task-${t.id}`}>
          {t.name}
        </button>
      ))}
    </div>
  ),
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

const websocketStoreState: {
  latestEvent: { type: string; data: unknown } | null;
} = {
  latestEvent: null,
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

function createWorkflow(overrides: Partial<Workflow> = {}): Workflow {
  return {
    id: 'wf-1',
    name: 'Merge workflow',
    description: 'Test workflow',
    state: 'running',
    tasks: [],
    metadata: {},
    createdAt: '2026-05-16T00:00:00.000Z',
    updatedAt: '2026-05-16T00:00:00.000Z',
    ...overrides,
  };
}

describe('WorkflowDetailPage', () => {
  beforeEach(() => {
    push.mockReset();
    replace.mockReset();
    workflowStoreState.fetchWorkflow.mockReset();
    workflowStoreState.executeWorkflow.mockReset();
    workflowStoreState.updateTaskState.mockReset();
    websocketStoreState.latestEvent = null;
    workflowStoreState.selectedWorkflow = null;
    workflowStoreState.isLoading = false;
  });

  it('shows loading spinner when loading and no workflow', () => {
    workflowStoreState.isLoading = true;

    render(<WorkflowDetailPage />);

    // Spinner element - look for the spinner by its animation class
    expect(document.querySelector('.animate-spin')).toBeInTheDocument();
  });

  it('shows "Workflow not found" when no workflow is loaded', () => {
    render(<WorkflowDetailPage />);

    expect(screen.getByText('Workflow not found')).toBeInTheDocument();
  });

  it('calls fetchWorkflow on mount with the workflow ID', () => {
    render(<WorkflowDetailPage />);

    expect(workflowStoreState.fetchWorkflow).toHaveBeenCalledWith('wf-1');
  });

  it('renders workflow name and description', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      name: 'My Workflow',
      description: 'A test workflow description',
      tasks: [],
    });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('My Workflow')).toBeInTheDocument();
    expect(screen.getByText('A test workflow description')).toBeInTheDocument();
  });

  it('renders workflow state badge', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ state: 'EXECUTE' });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('EXECUTE')).toBeInTheDocument();
  });

  it('shows DONE state with emerald styling', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ state: 'DONE' });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('DONE')).toBeInTheDocument();
  });

  it('shows FAILED state with red styling', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ state: 'FAILED' });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('FAILED')).toBeInTheDocument();
  });

  it('calls executeWorkflow when Execute button is clicked', async () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [] });
    workflowStoreState.executeWorkflow.mockResolvedValue(undefined);

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getByRole('button', { name: /execute/i }));

    await waitFor(() => {
      expect(workflowStoreState.executeWorkflow).toHaveBeenCalledWith('wf-1');
      expect(workflowStoreState.fetchWorkflow).toHaveBeenCalledWith('wf-1');
    });
  });

  it('shows task count and progress bar', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [
        createTask({ id: 't1', state: 'done' }),
        createTask({ id: 't2', state: 'done' }),
        createTask({ id: 't3', state: 'running' }),
        createTask({ id: 't4', state: 'pending' }),
      ],
    });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('2/4 tasks')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
  });

  it('shows 0% progress when no tasks', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [] });

    render(<WorkflowDetailPage />);

    expect(screen.getByText('0/0 tasks')).toBeInTheDocument();
    expect(screen.getByText('0%')).toBeInTheDocument();
  });

  it('renders tasks in the DataTable', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [
        createTask({ id: 't1', name: 'Build', state: 'done', agent: 'builder', skill: 'build' }),
        createTask({ id: 't2', name: 'Test', state: 'running', agent: 'tester', skill: 'test' }),
      ],
    });

    render(<WorkflowDetailPage />);

    expect(screen.getAllByText('Build')[0]).toBeInTheDocument();
    expect(screen.getAllByText('Test')[0]).toBeInTheDocument();
    expect(screen.getAllByText('builder')[0]).toBeInTheDocument();
    expect(screen.getAllByText('tester')[0]).toBeInTheDocument();
  });

  it('shows task state badges with correct colors', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [
        createTask({ id: 't1', state: 'done' }),
        createTask({ id: 't2', state: 'failed' }),
        createTask({ id: 't3', state: 'running' }),
        createTask({ id: 't4', state: 'pending' }),
      ],
    });

    render(<WorkflowDetailPage />);

    expect(screen.getAllByText('done')[0]).toBeInTheDocument();
    expect(screen.getAllByText('failed')[0]).toBeInTheDocument();
    expect(screen.getAllByText('running')[0]).toBeInTheDocument();
    expect(screen.getAllByText('pending')[0]).toBeInTheDocument();
  });

  it('selects a task when clicking on a row', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({ id: 't1', name: 'Selected Task' })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByText('Selected Task')[0]);

    expect(screen.getAllByText('Selected Task').length).toBeGreaterThan(0);
  });

  it('selects a task when clicking on a DAG node', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({ id: 't1', name: 'DAG Task' })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getByTestId('dag-task-t1'));

    // Task detail panel should appear
    expect(screen.getAllByText('DAG Task')[0]).toBeInTheDocument();
  });

  it('shows selected task details panel', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({
        id: 't1',
        name: 'Detail Task',
        state: 'running',
        agent: 'executor',
        skill: 'coding',
        dependencies: ['dep-1', 'dep-2'],
      })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByText('Detail Task')[0]);

    expect(screen.getByText((_, el) => el?.textContent === 'State: running')).toBeInTheDocument();
    expect(screen.getByText((_, el) => el?.textContent === 'Agent: executor')).toBeInTheDocument();
    expect(screen.getByText((_, el) => el?.textContent === 'Skill: coding')).toBeInTheDocument();
    expect(screen.getByText((_, el) => el?.textContent === 'Dependencies: 2')).toBeInTheDocument();
  });

  it('shows task output when available', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({
        id: 't1',
        name: 'Output Task',
        output: { result: 'success', files: ['a.ts', 'b.ts'] },
      })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByText('Output Task')[0]);

    expect(screen.getByText(/Output:/)).toBeInTheDocument();
    expect(screen.getByText(/"result": "success"/)).toBeInTheDocument();
  });

  it('does not show output section when task has no output', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({ id: 't1', name: 'No Output' })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(within(screen.getByRole('table')).getByText('No Output'));

    expect(screen.queryByText(/Output:/)).not.toBeInTheDocument();
  });

  it('closes task detail panel when Close button is clicked', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({
      tasks: [createTask({ id: 't1', name: 'Closeable Task' })],
    });

    render(<WorkflowDetailPage />);

    fireEvent.click(within(screen.getByRole('table')).getByText('Closeable Task'));
    expect(screen.getByText('Close')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Close'));

    // Task detail should be gone (only the table row remains)
    const closeButtons = screen.queryAllByText('Close');
    expect(closeButtons).toHaveLength(0);
  });

  it('navigates back to workflows list when back button is clicked', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [] });

    render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByRole('button')[0]);

    expect(push).toHaveBeenCalledWith('/workflows');
  });

  it('keeps selected task details in sync with the latest workflow task state', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ state: 'pending' })] });

    const { rerender } = render(<WorkflowDetailPage />);

    fireEvent.click(screen.getAllByText('Plan merge')[0]);
    expect(screen.getByText((_, element) => element?.textContent === 'State: pending')).toBeInTheDocument();

    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ state: 'running' })] });
    rerender(<WorkflowDetailPage />);

    expect(screen.getByText((_, element) => element?.textContent === 'State: running')).toBeInTheDocument();
    expect(screen.queryByText((_, element) => element?.textContent === 'State: pending')).not.toBeInTheDocument();
  });

  it('ignores task.* websocket events for other workflows when workflowId is present', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ state: 'pending' })] });

    const { rerender } = render(<WorkflowDetailPage />);

    websocketStoreState.latestEvent = {
      type: 'task.updated',
      data: { workflowId: 'wf-other', taskId: 'task-1', state: 'running' },
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).not.toHaveBeenCalled();
  });

  it('applies task.* websocket events without workflowId only when the task belongs to the selected workflow', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ id: 'task-1', state: 'pending' })] });

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

  it('ignores non-task websocket events', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ state: 'pending' })] });

    const { rerender } = render(<WorkflowDetailPage />);

    websocketStoreState.latestEvent = {
      type: 'workflow.updated',
      data: { workflowId: 'wf-1', state: 'done' },
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).not.toHaveBeenCalled();
  });

  it('ignores task events without taskId or state', () => {
    workflowStoreState.selectedWorkflow = createWorkflow({ tasks: [createTask({ state: 'pending' })] });

    const { rerender } = render(<WorkflowDetailPage />);

    websocketStoreState.latestEvent = {
      type: 'task.updated',
      data: { workflowId: 'wf-1' }, // missing taskId and state
    };
    rerender(<WorkflowDetailPage />);

    expect(workflowStoreState.updateTaskState).not.toHaveBeenCalled();
  });
});
