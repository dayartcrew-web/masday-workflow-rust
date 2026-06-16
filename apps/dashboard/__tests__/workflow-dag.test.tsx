import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
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

    expect(document.querySelector('svg')).toBeInTheDocument();
  });

  it('renders task names in the SVG', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getByText('Task 1')).toBeInTheDocument();
    expect(screen.getByText('Task 2')).toBeInTheDocument();
  });

  it('renders task states', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    // States appear in both task nodes and the legend.
    expect(screen.getAllByText('pending').length).toBeGreaterThan(0);
    expect(screen.getAllByText('running').length).toBeGreaterThan(0);
    expect(screen.getAllByText('done').length).toBeGreaterThan(0);
    expect(screen.getAllByText('failed').length).toBeGreaterThan(0);
  });

  it('shows empty message when no tasks', () => {
    render(<WorkflowDag tasks={[]} />);

    expect(screen.getByText('No tasks to display')).toBeInTheDocument();
  });

  it('calls onTaskClick when a task node is clicked', () => {
    const onTaskClick = vi.fn();
    render(<WorkflowDag tasks={mockTasks} onTaskClick={onTaskClick} />);

    fireEvent.click(screen.getByText('Task 1'));
    expect(onTaskClick).toHaveBeenCalledWith(mockTasks[0]);
  });

  it('renders legend with state colors', () => {
    render(<WorkflowDag tasks={mockTasks} />);

    expect(screen.getAllByText('pending').length).toBeGreaterThan(0);
    expect(screen.getAllByText('running').length).toBeGreaterThan(0);
    expect(screen.getAllByText('done').length).toBeGreaterThan(0);
    expect(screen.getAllByText('failed').length).toBeGreaterThan(0);
  });
});
