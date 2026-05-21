import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useWorkflowStore } from '@/stores/workflow-store';
import { workflowApi } from '@/lib/api-client';

vi.mock('@/lib/api-client', () => ({
  workflowApi: {
    list: vi.fn(),
    get: vi.fn(),
    getActive: vi.fn(),
    create: vi.fn(),
    execute: vi.fn(),
    getTasks: vi.fn(),
    addTask: vi.fn(),
  },
}));

describe('useWorkflowStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useWorkflowStore.setState({
      workflows: [],
      activeWorkflow: null,
      selectedWorkflow: null,
      tasks: [],
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with empty workflows and null selections', () => {
    const state = useWorkflowStore.getState();
    expect(state.workflows).toEqual([]);
    expect(state.activeWorkflow).toBeNull();
    expect(state.selectedWorkflow).toBeNull();
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetchWorkflows loads workflows via workflowApi.list', async () => {
    const mockWorkflows = [
      { id: '1', name: 'Workflow 1', status: 'active' },
      { id: '2', name: 'Workflow 2', status: 'done' },
    ];
    (workflowApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({ workflows: mockWorkflows });

    await useWorkflowStore.getState().fetchWorkflows();

    const state = useWorkflowStore.getState();
    expect(state.workflows).toEqual(mockWorkflows);
    expect(state.isLoading).toBe(false);
  });

  it('fetchWorkflows sets error on failure', async () => {
    (workflowApi.list as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Network error'));

    await useWorkflowStore.getState().fetchWorkflows();

    expect(useWorkflowStore.getState().error).toBe('Network error');
    expect(useWorkflowStore.getState().isLoading).toBe(false);
  });

  it('fetchWorkflow loads and stores selected workflow', async () => {
    const mockWorkflow = { id: '1', name: 'Detail', tasks: [] };
    (workflowApi.get as ReturnType<typeof vi.fn>).mockResolvedValue({ workflow: mockWorkflow });

    await useWorkflowStore.getState().fetchWorkflow('1');

    expect(useWorkflowStore.getState().selectedWorkflow).toEqual(mockWorkflow);
    expect(useWorkflowStore.getState().tasks).toEqual([]);
  });

  it('fetchActive sets activeWorkflow', async () => {
    const mockActive = { id: 'active-1', name: 'Active Workflow' };
    (workflowApi.getActive as ReturnType<typeof vi.fn>).mockResolvedValue({ workflow: mockActive });

    await useWorkflowStore.getState().fetchActive();

    expect(useWorkflowStore.getState().activeWorkflow).toEqual(mockActive);
  });

  it('createWorkflow adds new workflow to list', async () => {
    const newWorkflow = { id: 'new', name: 'Created Workflow' };
    (workflowApi.create as ReturnType<typeof vi.fn>).mockResolvedValue({ workflow: newWorkflow });

    const result = await useWorkflowStore.getState().createWorkflow('New Workflow', 'Description');

    const state = useWorkflowStore.getState();
    expect(state.workflows).toContainEqual(newWorkflow);
    expect(result).toEqual(newWorkflow);
  });

  it('executeWorkflow updates workflow in list', async () => {
    const existingWorkflow = { id: '1', name: 'To Execute', status: 'pending' };
    useWorkflowStore.setState({ workflows: [existingWorkflow] });

    const updatedWorkflow = { id: '1', name: 'To Execute', status: 'running' };
    (workflowApi.execute as ReturnType<typeof vi.fn>).mockResolvedValue({ workflow: updatedWorkflow });

    await useWorkflowStore.getState().executeWorkflow('1');

    expect(workflowApi.execute).toHaveBeenCalledWith('1');
    expect(useWorkflowStore.getState().selectedWorkflow).toEqual(updatedWorkflow);
  });

  it('fetchTasks loads tasks for a workflow', async () => {
    const mockTasks = [
      { id: 't1', name: 'Task 1', state: 'PENDING' },
      { id: 't2', name: 'Task 2', state: 'DONE' },
    ];
    (workflowApi.getTasks as ReturnType<typeof vi.fn>).mockResolvedValue({ tasks: mockTasks });

    await useWorkflowStore.getState().fetchTasks('wf-1');

    expect(useWorkflowStore.getState().tasks).toEqual(mockTasks);
  });

  it('updateTaskState updates task state in tasks array', () => {
    useWorkflowStore.setState({
      tasks: [
        { id: 't1', name: 'Task 1', state: 'PENDING' },
        { id: 't2', name: 'Task 2', state: 'PENDING' },
      ],
    });

    useWorkflowStore.getState().updateTaskState('wf-1', 't1', 'RUNNING');

    const tasks = useWorkflowStore.getState().tasks;
    expect(tasks.find(t => t.id === 't1')?.state).toBe('RUNNING');
    expect(tasks.find(t => t.id === 't2')?.state).toBe('PENDING');
  });

  it('clearError resets error state', () => {
    useWorkflowStore.setState({ error: 'Some error' });

    useWorkflowStore.getState().clearError();

    expect(useWorkflowStore.getState().error).toBeNull();
  });
});
