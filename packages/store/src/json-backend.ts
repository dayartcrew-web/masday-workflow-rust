import fs from 'fs';
import path from 'path';
import type { StorageBackend, RunResult } from './types.js';
import { createLogger } from '@mcp-rebuild/core';

const logger = createLogger('JsonBackend');

interface JsonData {
  [table: string]: Record<string, Record<string, unknown>>;
}

export class JsonBackend implements StorageBackend {
  private filePath: string;
  private data: JsonData;
  private seqCounters: Record<string, number>;

  constructor(filePath: string) {
    this.filePath = filePath;
    this.data = {};
    this.seqCounters = {};
  }

  initialize(): void {
    try {
      const dir = path.dirname(this.filePath);
      if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
      }

      if (fs.existsSync(this.filePath)) {
        const content = fs.readFileSync(this.filePath, 'utf-8');
        this.data = JSON.parse(content);
      }
      // Rebuild seq counters from existing data
      for (const table of Object.keys(this.data)) {
        const rows = Object.values(this.data[table]);
        const maxSeq = rows.reduce((max, row) => {
          const seq = typeof row.seq === 'number' ? row.seq : 0;
          return Math.max(max, seq);
        }, 0);
        this.seqCounters[table] = maxSeq;
      }
      logger.info(`JSON backend initialized at ${this.filePath}`);
    } catch {
      this.data = {};
      this.seqCounters = {};
      logger.warn('Failed to load existing JSON data, starting fresh');
    }
  }

  close(): void {
    this.save();
    logger.info('JSON backend closed');
  }

  run(sql: string, params?: unknown[]): RunResult {
    const parsed = this.parseSql(sql, params);
    if (!this.data[parsed.table]) {
      this.data[parsed.table] = {};
    }

    if (parsed.operation === 'INSERT' || parsed.operation === 'REPLACE') {
      const pk = parsed.primaryKey;
      if (!pk || !parsed.values) return { changes: 0, lastInsertRowid: 0 };
      // Auto-assign seq for tables that use auto-increment
      if (!(parsed.table in this.seqCounters)) {
        this.seqCounters[parsed.table] = 0;
      }
      this.seqCounters[parsed.table]++;
      parsed.values['seq'] = this.seqCounters[parsed.table];
      this.data[parsed.table][pk] = parsed.values;
      this.save();
      return { changes: 1, lastInsertRowid: this.seqCounters[parsed.table] };
    }

    if (parsed.operation === 'UPDATE') {
      let changes = 0;
      const table = this.data[parsed.table] || {};
      for (const key of Object.keys(table)) {
        if (parsed.matches(table[key])) {
          Object.assign(table[key], parsed.updates);
          changes++;
        }
      }
      if (changes > 0) this.save();
      return { changes, lastInsertRowid: 0 };
    }

    if (parsed.operation === 'DELETE') {
      let changes = 0;
      const table = this.data[parsed.table] || {};
      if (parsed.deleteKey) {
        if (table[parsed.deleteKey]) {
          delete table[parsed.deleteKey];
          changes = 1;
        }
      } else {
        changes = Object.keys(table).length;
        this.data[parsed.table] = {};
      }
      if (changes > 0) this.save();
      return { changes, lastInsertRowid: 0 };
    }

    return { changes: 0, lastInsertRowid: 0 };
  }

  query<T = Record<string, unknown>>(sql: string, params?: unknown[]): T[] {
    const parsed = this.parseSql(sql, params);
    const table = this.data[parsed.table] || {};
    let rows = Object.values(table).filter(row => parsed.matches(row));

    // Apply ORDER BY
    if (parsed.orderBy) {
      const col = parsed.orderBy.column;
      const dir = parsed.orderBy.direction;
      rows.sort((a, b) => {
        const va = a[col] ?? '';
        const vb = b[col] ?? '';
        const cmp = String(va).localeCompare(String(vb), undefined, { numeric: true });
        return dir === 'DESC' ? -cmp : cmp;
      });
    }

    // Apply LIMIT
    if (parsed.limit !== null) {
      rows = rows.slice(0, parsed.limit);
    }

    return rows as T[];
  }

  queryOne<T = Record<string, unknown>>(sql: string, params?: unknown[]): T | undefined {
    const rows = this.query<T>(sql, params);
    return rows[0];
  }

  private save(): void {
    const dir = path.dirname(this.filePath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    fs.writeFileSync(this.filePath, JSON.stringify(this.data, null, 2), 'utf-8');
  }

  private parseSql(sql: string, params?: unknown[]): ParsedSql {
    const normalized = sql.trim().toUpperCase();

    if (normalized.startsWith('INSERT') || normalized.startsWith('REPLACE')) {
      return this.parseInsert(sql, params);
    }
    if (normalized.startsWith('UPDATE')) {
      return this.parseUpdate(sql, params);
    }
    if (normalized.startsWith('DELETE')) {
      return this.parseDelete(sql, params);
    }
    if (normalized.startsWith('SELECT')) {
      return this.parseSelect(sql, params);
    }

    return { operation: 'UNKNOWN', table: '', matches: () => false };
  }

  private parseInsert(sql: string, params?: unknown[]): ParsedSql {
    const tableMatch = sql.match(/INTO\s+(\w+)/i);
    const table = tableMatch ? tableMatch[1] : '';
    const placeholders = sql.match(/\(\s*([^)]+)\s*\)\s*VALUES/i);
    const columns = placeholders
      ? placeholders[1].split(',').map(c => c.trim())
      : [];
    const values = (params || []) as unknown[];

    const row: Record<string, unknown> = {};
    columns.forEach((col, i) => {
      row[col] = values[i];
    });

    const primaryKey = String(values[0]);
    const operation = sql.trim().toUpperCase().startsWith('REPLACE') ? 'REPLACE' : 'INSERT';

    return { operation, table, primaryKey, values: row, matches: () => true };
  }

  private parseUpdate(sql: string, params?: unknown[]): ParsedSql {
    const tableMatch = sql.match(/UPDATE\s+(\w+)/i);
    const table = tableMatch ? tableMatch[1] : '';
    const setMatch = sql.match(/SET\s+(.+?)(?:\s+WHERE|$)/i);
    const updates: Record<string, unknown> = {};
    let placeholderCount = 0;

    if (setMatch) {
      const setClauses = setMatch[1].split(',').map(s => s.trim());

      for (const clause of setClauses) {
        // Match placeholder: col = ?
        const placeholderMatch = clause.match(/(\w+)\s*=\s*\?/);
        if (placeholderMatch) {
          updates[placeholderMatch[1]] = (params || [])[placeholderCount];
          placeholderCount++;
          continue;
        }
        // Match literal value: col = 'value' or col = "value"
        const literalMatch = clause.match(/(\w+)\s*=\s*'([^']*)'/);
        if (literalMatch) {
          updates[literalMatch[1]] = literalMatch[2];
        }
      }
    }

    const whereMatch = sql.match(/WHERE\s+(\w+)\s*=\s*\?/i);
    const whereCol = whereMatch ? whereMatch[1] : null;
    const whereParamIdx = placeholderCount;

    return {
      operation: 'UPDATE',
      table,
      updates,
      matches: (row: Record<string, unknown>) => {
        if (!whereCol) return true;
        const expected = params ? params[whereParamIdx] : undefined;
        return row[whereCol] === expected;
      },
    };
  }

  private parseDelete(sql: string, params?: unknown[]): ParsedSql {
    const tableMatch = sql.match(/FROM\s+(\w+)/i);
    const table = tableMatch ? tableMatch[1] : '';
    const whereMatch = sql.match(/WHERE\s+(\w+)\s*=\s*\?/i);

    let deleteKey: string | null = null;
    if (whereMatch && params) {
      deleteKey = String(params[0]);
    }

    return {
      operation: 'DELETE',
      table,
      deleteKey,
      matches: () => true,
    };
  }

  private parseSelect(sql: string, params?: unknown[]): ParsedSql {
    const tableMatch = sql.match(/FROM\s+(\w+)/i);
    const table = tableMatch ? tableMatch[1] : '';

    // Parse multiple WHERE conditions: col1 = ? AND col2 = ?
    const whereClause = sql.match(/WHERE\s+(.+?)(?:\s+ORDER\s+BY|\s+LIMIT|$)/i);
    const conditions: Array<{ col: string; paramIdx: number }> = [];
    if (whereClause) {
      const condRegex = /(\w+)\s*=\s*\?/g;
      const clauseText = whereClause[1];
      let match: RegExpExecArray | null;
      let paramIdx = 0;
      while ((match = condRegex.exec(clauseText)) !== null) {
        conditions.push({ col: match[1], paramIdx });
        paramIdx++;
      }
    }

    // Parse ORDER BY
    let orderBy: { column: string; direction: 'ASC' | 'DESC' } | null = null;
    const orderMatch = sql.match(/ORDER\s+BY\s+(\w+)(?:\s+(ASC|DESC))?/i);
    if (orderMatch) {
      orderBy = {
        column: orderMatch[1],
        direction: (orderMatch[2]?.toUpperCase() === 'DESC' ? 'DESC' : 'ASC'),
      };
    }

    // Parse LIMIT
    let limit: number | null = null;
    const limitMatch = sql.match(/LIMIT\s+(\d+)/i);
    if (limitMatch) {
      limit = parseInt(limitMatch[1], 10);
    }

    return {
      operation: 'SELECT',
      table,
      orderBy,
      limit,
      matches: (row: Record<string, unknown>) => {
        if (conditions.length === 0 || !params || params.length === 0) return true;
        return conditions.every(cond => row[cond.col] === params[cond.paramIdx]);
      },
    };
  }
}

interface ParsedSql {
  operation: string;
  table: string;
  primaryKey?: string;
  values?: Record<string, unknown>;
  updates?: Record<string, unknown>;
  deleteKey?: string | null;
  orderBy?: { column: string; direction: 'ASC' | 'DESC' } | null;
  limit?: number | null;
  matches: (row: Record<string, unknown>) => boolean;
}
