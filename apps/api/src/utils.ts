// ============================================================
// Shared utilities for the API server
// ============================================================

import { IncomingMessage, ServerResponse } from 'http';

export class APIError extends Error {
  readonly statusCode: number;
  readonly code: string;

  constructor(statusCode: number, code: string, message: string) {
    super(message);
    this.statusCode = statusCode;
    this.code = code;
    this.name = 'APIError';
  }
}

/** Send a JSON response with status code */
export function sendJson(res: ServerResponse, status: number, data: unknown): void {
  const body = JSON.stringify(data);
  res.writeHead(status, {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(body),
  });
  res.end(body);
}

/** Read request body as string with size limit */
export function readBody(req: IncomingMessage, maxSize: number = 1_000_000): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    let totalSize = 0;
    let tooLarge = false;

    req.on('data', (chunk: Buffer) => {
      if (tooLarge) return;
      totalSize += chunk.length;
      if (totalSize > maxSize) {
        tooLarge = true;
        req.on('end', () => {
          reject(new APIError(413, 'PAYLOAD_TOO_LARGE', `Body exceeds ${maxSize} byte limit`));
        });
        return;
      }
      chunks.push(chunk);
    });

    req.on('end', () => {
      if (!tooLarge) resolve(Buffer.concat(chunks).toString());
    });

    req.on('error', reject);
  });
}

/** Parse JSON body */
export function parseJsonBody(body: string): Record<string, unknown> {
  try {
    return JSON.parse(body || '{}') as Record<string, unknown>;
  } catch {
    throw new APIError(400, 'INVALID_JSON', 'Request body must be valid JSON');
  }
}

/** Extract path parameters from a route pattern match */
export function extractPathParams(
  pathname: string,
  pattern: string,
): Record<string, string> | null {
  const patternParts = pattern.split('/');
  const pathParts = pathname.split('/');

  if (patternParts.length !== pathParts.length) return null;

  const params: Record<string, string> = {};
  for (let i = 0; i < patternParts.length; i++) {
    if (patternParts[i].startsWith(':')) {
      params[patternParts[i].slice(1)] = decodeURIComponent(pathParts[i]);
    } else if (patternParts[i] !== pathParts[i]) {
      return null;
    }
  }
  return params;
}

/** Check if a pathname matches a route pattern */
export function matchRoute(
  method: string,
  pathname: string,
  expectedMethod: string,
  pattern: string,
): Record<string, string> | null {
  if (method !== expectedMethod) return null;
  return extractPathParams(pathname, pattern);
}

/** Type-safe string accessor */
export function str(val: unknown, fallback: string = ''): string {
  return typeof val === 'string' ? val : fallback;
}

/** Type-safe optional string accessor */
export function optStr(val: unknown): string | undefined {
  return typeof val === 'string' ? val : undefined;
}

/** Type-safe number accessor */
export function num(val: unknown, fallback: number = 0): number {
  return typeof val === 'number' && !Number.isNaN(val) ? val : fallback;
}

/** Get client IP from request */
export function getClientIp(req: IncomingMessage): string {
  const forwarded = req.headers['x-forwarded-for'];
  if (typeof forwarded === 'string') return forwarded.split(',')[0].trim();
  if (Array.isArray(forwarded)) return forwarded[0].trim();
  return req.socket.remoteAddress || 'unknown';
}

/** Route handler function type */
export type RouteHandler = (
  req: IncomingMessage,
  res: ServerResponse,
  params: Record<string, string>,
  body?: Record<string, unknown>,
  query?: URLSearchParams,
) => Promise<void>;

/** Route definition */
export interface RouteDefinition {
  method: string;
  pattern: string;
  handler: RouteHandler;
  authRequired?: boolean;
  roles?: string[];
  rateLimit?: { windowMs: number; max: number };
}
