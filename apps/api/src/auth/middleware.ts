// ============================================================
// Auth middleware — extracts and validates user from requests
// ============================================================

import { IncomingMessage } from 'http';
import { authenticateRequest, hasRole } from './jwt';
import type { AuthUser, TokenPayload } from './jwt';
import { APIError } from '../utils';

export interface AuthResult {
  user: AuthUser;
  payload: TokenPayload;
}

/** Require authentication from a request */
export function requireAuth(req: IncomingMessage): AuthResult {
  const authHeader = req.headers['authorization'] as string | undefined;
  const result = authenticateRequest(authHeader);

  if (!result) {
    throw new APIError(401, 'UNAUTHORIZED', 'Authentication required');
  }

  return result;
}

/** Require specific roles */
export function requireRoles(req: IncomingMessage, roles: string[]): AuthResult {
  const auth = requireAuth(req);

  if (!hasRole(auth.user, roles)) {
    throw new APIError(403, 'FORBIDDEN', 'Insufficient permissions');
  }

  return auth;
}
