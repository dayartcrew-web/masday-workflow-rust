import { Task, TaskState } from './types.js';

export class TaskManager {
  private tasks: Map<string, Task> = new Map();

  create(task: Omit<Task, 'id' | 'createdAt' | 'state'>): Task {
    const newTask: Task = {
      ...task,
      id: this.generateId(),
      state: 'pending',
      createdAt: new Date(),
    };
    this.tasks.set(newTask.id, newTask);
    return newTask;
  }

  get(id: string): Task | undefined {
    return this.tasks.get(id);
  }

  getAll(): Task[] {
    return Array.from(this.tasks.values());
  }

  updateState(id: string, state: TaskState): Task | undefined {
    const task = this.tasks.get(id);
    if (!task) return undefined;

    task.state = state;
    if (state === 'running') {
      task.startedAt = new Date();
    } else if (state === 'done' || state === 'failed') {
      task.completedAt = new Date();
    }

    return task;
  }

  setOutput(id: string, output: unknown): Task | undefined {
    const task = this.tasks.get(id);
    if (!task) return undefined;

    task.output = output;
    return task;
  }

  setError(id: string, error: string): Task | undefined {
    const task = this.tasks.get(id);
    if (!task) return undefined;

    task.error = error;
    return task;
  }

  getPendingTasks(): Task[] {
    return this.getAll().filter(t => t.state === 'pending');
  }

  getReadyTasks(allTasks: Task[]): Task[] {
    const taskMap = new Map(allTasks.map(t => [t.id, t]));
    return allTasks
      .filter(t => t.state === 'pending')
      .filter(t => t.dependencies.every(depId => {
        const depTask = taskMap.get(depId);
        return depTask && depTask.state === 'done';
      }));
  }

  private generateId(): string {
    return `task_${Date.now()}_${Math.random().toString(36).substring(7)}`;
  }
}
