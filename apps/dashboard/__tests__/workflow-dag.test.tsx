import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { WorkflowDag } from '@/components/workflow-dag';

describe('WorkflowDag', () => {
  const mockTasks = [
    { id: '1', name: 'Task 1', state: 'pending' as const, dependencies: [] },
    { id: '2', name: 'Task 2', state: 'running' as const, dependencies: ['1'] },
    { id: '3', name: 'Task 3', state: 'done' as const, dependencies: ['1'] },
    { id: '4', name: 'Task 4', state: 'failed' as const, dependencies: ['2', '3'] },
  ];

  it('renders SVG element', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getByRole('img', { hidden: true })?.tagName || document.querySelector('svg')).toBeTruthy();
    expect(document.querySelector('svg')).toBeInTheDocument();
  });

  it('renders task names in the SVG', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getByText('Task 1')).toBeInTheDocument();
    expect(screen.getByText('Task 2')).toBeInTheDocument();
  });

  it('renders task states', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getByText('pending')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText('done')).toBeInTheDocument();
    expect(screen.getByText('failed')).toBeInTheDocument();
  });

  it('shows empty message when no tasks', () => {
    render(<WorkflowDag tasks={[]} />);

    expect(screen.getByText('No tasks to display')).toBeInTheDocument();
  });

  it('calls onTaskClick when a task node is clicked', () => {
    const onTaskClick = vi.fn();
    render(<WorkflowDag tasks={mockTasks} onTaskClick={onTaskClick} />);

    screen.getByText('Task 1').click();
    expect(onTaskClick).toHaveBeenCalledWith(mockTasks[0]);
  });

  it('renders legend with state colors', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getByText('pending')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText('done')).toBeInTheDocument();
    expect(screen.getByText('failed')).toBeInTheDocument();
  });
});
