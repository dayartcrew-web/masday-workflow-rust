import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DataTable } from '@/components/ui/data-table';

interface Row {
  id: string;
  name: string;
  status: string;
}

const columns = [
  { key: 'name', label: 'Name', sortable: true },
  { key: 'status', label: 'Status', sortable: true },
];

function createRow(overrides: Partial<Row> = {}): Row {
  return {
    id: `row-${Math.random()}`,
    name: 'Item',
    status: 'active',
    ...overrides,
  };
}

describe('DataTable', () => {
  it('clamps back to a valid page when the dataset shrinks', () => {
    const { rerender } = render(
      <DataTable<Row>
        columns={columns}
        data={[
          createRow({ id: '1', name: 'Alpha' }),
          createRow({ id: '2', name: 'Beta' }),
        ]}
        keyField="id"
        pageSize={1}
        emptyMessage="No rows"
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(screen.getByText('Beta')).toBeInTheDocument();

    rerender(
      <DataTable<Row>
        columns={columns}
        data={[createRow({ id: '1', name: 'Alpha' })]}
        keyField="id"
        pageSize={1}
        emptyMessage="No rows"
      />,
    );

    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('No rows')).not.toBeInTheDocument();
  });

  it('shows empty message when data is empty', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[]}
        keyField="id"
        emptyMessage="Nothing here"
      />,
    );

    expect(screen.getByText('Nothing here')).toBeInTheDocument();
  });

  it('renders all columns in the header', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[createRow({ id: '1', name: 'Test', status: 'active' })]}
        keyField="id"
      />,
    );

    expect(screen.getByText('Name')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
  });

  it('renders data rows with correct values', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[
          createRow({ id: '1', name: 'Alpha', status: 'active' }),
          createRow({ id: '2', name: 'Beta', status: 'inactive' }),
        ]}
        keyField="id"
      />,
    );

    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
    expect(screen.getByText('active')).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
  });

  it('supports custom render function for columns', () => {
    const customColumns = [
      {
        key: 'name',
        label: 'Name',
        render: (item: Row) => <span data-testid="custom-name">{item.name.toUpperCase()}</span>,
      },
    ];

    render(
      <DataTable<Row>
        columns={customColumns}
        data={[createRow({ id: '1', name: 'hello' })]}
        keyField="id"
      />,
    );

    expect(screen.getByTestId('custom-name')).toHaveTextContent('HELLO');
  });

  it('sorts ascending when clicking sortable column header', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[
          createRow({ id: '1', name: 'Charlie', status: 'active' }),
          createRow({ id: '2', name: 'Alpha', status: 'active' }),
          createRow({ id: '3', name: 'Beta', status: 'active' }),
        ]}
        keyField="id"
        pageSize={10}
      />,
    );

    // Click Name header to sort
    fireEvent.click(screen.getByText('Name'));

    // Should be sorted ascending: Alpha, Beta, Charlie
    const rows = screen.getAllByRole('row');
    // First data row (after header) should be Alpha
    expect(rows[1]).toHaveTextContent('Alpha');
  });

  it('toggles to descending when clicking same sortable column twice', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[
          createRow({ id: '1', name: 'Alpha', status: 'active' }),
          createRow({ id: '2', name: 'Charlie', status: 'active' }),
          createRow({ id: '3', name: 'Beta', status: 'active' }),
        ]}
        keyField="id"
        pageSize={10}
      />,
    );

    // First click: ascending
    fireEvent.click(screen.getByText('Name'));
    let rows = screen.getAllByRole('row');
    expect(rows[1]).toHaveTextContent('Alpha');

    // Second click: descending
    fireEvent.click(screen.getByText('Name'));
    rows = screen.getAllByRole('row');
    expect(rows[1]).toHaveTextContent('Charlie');
  });

  it('does not sort when clicking non-sortable column', () => {
    const nonSortableColumns = [
      { key: 'name', label: 'Name' },
      { key: 'status', label: 'Status' },
    ];

    const { container } = render(
      <DataTable<Row>
        columns={nonSortableColumns}
        data={[
          createRow({ id: '1', name: 'Charlie' }),
          createRow({ id: '2', name: 'Alpha' }),
        ]}
        keyField="id"
        pageSize={10}
      />,
    );

    // Click Name header - should not sort (no cursor-pointer class)
    const nameHeader = screen.getByText('Name');
    expect(nameHeader.closest('th')).not.toHaveAttribute('onclick');

    // Order should remain unchanged (Charlie first)
    const rows = screen.getAllByRole('row');
    expect(rows[1]).toHaveTextContent('Charlie');
  });

  it('paginates data correctly with page size', () => {
    const data = Array.from({ length: 5 }, (_, i) =>
      createRow({ id: String(i + 1), name: `Item ${i + 1}` }),
    );

    render(
      <DataTable<Row>
        columns={columns}
        data={data}
        keyField="id"
        pageSize={2}
      />,
    );

    // First page: Item 1, Item 2
    expect(screen.getByText('Item 1')).toBeInTheDocument();
    expect(screen.getByText('Item 2')).toBeInTheDocument();
    expect(screen.queryByText('Item 3')).not.toBeInTheDocument();

    // Navigate to next page
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    // Second page: Item 3, Item 4
    expect(screen.getByText('Item 3')).toBeInTheDocument();
    expect(screen.getByText('Item 4')).toBeInTheDocument();
    expect(screen.queryByText('Item 1')).not.toBeInTheDocument();
  });

  it('shows page info when multiple pages exist', () => {
    const data = Array.from({ length: 3 }, (_, i) =>
      createRow({ id: String(i + 1), name: `Item ${i + 1}` }),
    );

    render(
      <DataTable<Row>
        columns={columns}
        data={data}
        keyField="id"
        pageSize={1}
      />,
    );

    expect(screen.getByText('Page 1 of 3')).toBeInTheDocument();
  });

  it('does not show pagination when all data fits on one page', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[createRow({ id: '1', name: 'Single' })]}
        keyField="id"
        pageSize={10}
      />,
    );

    expect(screen.queryByRole('button', { name: 'Next' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Prev' })).not.toBeInTheDocument();
  });

  it('disables Prev button on first page', () => {
    const data = Array.from({ length: 3 }, (_, i) =>
      createRow({ id: String(i + 1), name: `Item ${i + 1}` }),
    );

    render(
      <DataTable<Row>
        columns={columns}
        data={data}
        keyField="id"
        pageSize={1}
      />,
    );

    const prevButton = screen.getByRole('button', { name: 'Prev' });
    expect(prevButton).toBeDisabled();
  });

  it('disables Next button on last page', () => {
    const data = Array.from({ length: 2 }, (_, i) =>
      createRow({ id: String(i + 1), name: `Item ${i + 1}` }),
    );

    render(
      <DataTable<Row>
        columns={columns}
        data={data}
        keyField="id"
        pageSize={1}
      />,
    );

    // Go to last page
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));

    const nextButton = screen.getByRole('button', { name: 'Next' });
    expect(nextButton).toBeDisabled();
  });

  it('calls onRowClick when a row is clicked', () => {
    const onRowClick = vi.fn();
    const row = createRow({ id: '1', name: 'Clickable', status: 'active' });

    render(
      <DataTable<Row>
        columns={columns}
        data={[row]}
        keyField="id"
        onRowClick={onRowClick}
      />,
    );

    fireEvent.click(screen.getByText('Clickable'));
    expect(onRowClick).toHaveBeenCalledWith(row);
  });

  it('does not call onRowClick when not provided', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[createRow({ id: '1', name: 'No Click' })]}
        keyField="id"
      />,
    );

    // Row should not have cursor-pointer style (no onRowClick)
    expect(screen.queryByText('No Click')).toBeInTheDocument();
  });

  it('handles null values gracefully in sorting', () => {
    const dataWithNulls: Row[] = [
      createRow({ id: '1', name: 'Alpha', status: 'active' }),
      { id: '2', name: null as unknown as string, status: 'active' },
      createRow({ id: '3', name: 'Beta', status: 'active' }),
    ];

    render(
      <DataTable<Row>
        columns={columns}
        data={dataWithNulls}
        keyField="id"
        pageSize={10}
      />,
    );

    // Should not crash, all items rendered
    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
  });

  it('uses default page size of 20', () => {
    const data = Array.from({ length: 21 }, (_, i) =>
      createRow({ id: String(i + 1), name: `Item ${i + 1}` }),
    );

    render(
      <DataTable<Row>
        columns={columns}
        data={data}
        keyField="id"
      />,
    );

    // First 20 items visible
    expect(screen.getByText('Item 1')).toBeInTheDocument();
    expect(screen.getByText('Item 20')).toBeInTheDocument();
    expect(screen.queryByText('Item 21')).not.toBeInTheDocument();
  });

  it('uses default keyField of "id"', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[createRow({ id: 'unique-key', name: 'Test' })]}
        keyField="id"
      />,
    );

    expect(screen.getByText('Test')).toBeInTheDocument();
  });

  it('uses default empty message', () => {
    render(
      <DataTable<Row>
        columns={columns}
        data={[]}
        keyField="id"
      />,
    );

    expect(screen.getByText('No data available')).toBeInTheDocument();
  });
});
