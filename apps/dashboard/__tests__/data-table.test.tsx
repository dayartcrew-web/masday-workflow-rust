import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DataTable } from '@/components/ui/data-table';

interface Row {
  id: string;
  name: string;
}

describe('DataTable', () => {
  it('clamps back to a valid page when the dataset shrinks', () => {
    const { rerender } = render(
      <DataTable<Row>
        columns={[{ key: 'name', label: 'Name' }]}
        data={[
          { id: '1', name: 'Alpha' },
          { id: '2', name: 'Beta' },
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
        columns={[{ key: 'name', label: 'Name' }]}
        data={[{ id: '1', name: 'Alpha' }]}
        keyField="id"
        pageSize={1}
        emptyMessage="No rows"
      />,
    );

    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.queryByText('No rows')).not.toBeInTheDocument();
  });
});
