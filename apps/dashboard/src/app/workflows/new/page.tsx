'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { AppShell } from '@/components/app-shell';
import { useWorkflowStore } from '@/stores/workflow-store';
import { ArrowLeft, Plus, Trash2 } from 'lucide-react';

interface TaskInput {
  name: string;
  agent: string;
  skill: string;
  dependencies: string[];
}

export default function NewWorkflowPage() {
  const router = useRouter();
  const createWorkflow = useWorkflowStore((s) => s.createWorkflow);
  const addTask = useWorkflowStore((s) => s.addTask);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [tasks, setTasks] = useState<TaskInput[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState('');

  const addTaskRow = () => {
    setTasks([...tasks, { name: '', agent: 'default', skill: 'default', dependencies: [] }]);
  };

  const removeTaskRow = (idx: number) => {
    setTasks(tasks.filter((_, i) => i !== idx));
  };

  const updateTask = (idx: number, field: keyof TaskInput, value: string) => {
    const updated = [...tasks];
    updated[idx] = { ...updated[idx], [field]: value };
    setTasks(updated);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setIsSubmitting(true);
    try {
      const workflow = await createWorkflow(name, description);
      for (const task of tasks) {
        if (task.name.trim()) {
          await addTask(workflow.id, {
            name: task.name,
            agent: task.agent,
            skill: task.skill,
            dependencies: task.dependencies,
          });
        }
      }
      router.push(`/workflows/${workflow.id}`);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to create workflow');
      setIsSubmitting(false);
    }
  };

  return (
    <AppShell>
      <div className="max-w-2xl mx-auto space-y-6">
        <div className="flex items-center gap-3">
          <button onClick={() => router.push('/workflows')} className="p-2 rounded-lg hover:bg-[var(--bg-card)]">
            <ArrowLeft className="w-4 h-4 text-[var(--text-secondary)]" />
          </button>
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Create Workflow</h2>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="text-sm text-red-500 bg-red-500/10 rounded-lg px-3 py-2">{error}</div>
          )}

          <div>
            <label className="block text-sm font-medium text-[var(--text-secondary)] mb-1">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-[var(--text-secondary)] mb-1">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 resize-none"
            />
          </div>

          {/* Tasks */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium text-[var(--text-secondary)]">Tasks</label>
              <button type="button" onClick={addTaskRow} className="flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300">
                <Plus className="w-3 h-3" /> Add Task
              </button>
            </div>
            <div className="space-y-2">
              {tasks.map((task, idx) => (
                <div key={idx} className="flex flex-col sm:flex-row gap-2 items-start p-3 rounded-lg bg-[var(--bg-secondary)]">
                  <input
                    type="text"
                    value={task.name}
                    onChange={(e) => updateTask(idx, 'name', e.target.value)}
                    placeholder="Task name"
                    className="w-full sm:flex-1 px-3 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                  />
                  <div className="flex gap-2 w-full sm:w-auto">
                    <input
                      type="text"
                      value={task.agent}
                      onChange={(e) => updateTask(idx, 'agent', e.target.value)}
                      placeholder="Agent"
                      className="flex-1 sm:w-24 px-3 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                    />
                    <input
                      type="text"
                      value={task.skill}
                      onChange={(e) => updateTask(idx, 'skill', e.target.value)}
                      placeholder="Skill"
                      className="flex-1 sm:w-24 px-3 py-1.5 rounded-lg border border-[var(--border)] bg-[var(--bg-card)] text-[var(--text-primary)] text-sm focus:outline-none focus:ring-2 focus:ring-brand-500"
                    />
                    <button type="button" onClick={() => removeTaskRow(idx)} className="p-1.5 text-red-400 hover:text-red-300 flex-shrink-0">
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <button
            type="submit"
            disabled={isSubmitting || !name.trim()}
            className="w-full py-2 rounded-lg bg-brand-600 text-white font-medium text-sm hover:bg-brand-700 disabled:opacity-50 transition-colors"
          >
            {isSubmitting ? 'Creating...' : 'Create Workflow'}
          </button>
        </form>
      </div>
    </AppShell>
  );
}
