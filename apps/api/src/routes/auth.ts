// ============================================================
// Auth routes — login, token verification, user info
// ============================================================

import type { ServerResponse, IncomingMessage } from 'http';
import { createUser, getUserByEmail, signToken, verifyToken, authenticateRequest } from '../auth/jwt';
import { validateBody, loginSchema, tokenSchema } from '../validation';
import { sendJson } from '../utils';
import type { RouteDefinition } from '../utils';

export const authRoutes: RouteDefinition[] = [
  // POST /api/auth/login
  {
    method: 'POST',
    pattern: '/api/auth/login',
    handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
      const input = body!;
      const email = input.email as string;
      const name = input.name as string;

      const existing = getUserByEmail(email);
      const user = existing || createUser({ email, name });
      const token = signToken({ userId: user.id, email: user.email, role: user.role });

      sendJson(res, 200, {
        token,
        user: { id: user.id, email: user.email, name: user.name, role: user.role },
      });
    },
  },
  // POST /api/auth/token
  {
    method: 'POST',
    pattern: '/api/auth/token',
    handler: async (_req: IncomingMessage, res: ServerResponse, _params, body?: Record<string, unknown>) => {
      const input = body!;
      const tokenStr = input.token as string;
      const payload = verifyToken(tokenStr);

      if (!payload) {
        sendJson(res, 200, { valid: false, error: 'Invalid or expired token' });
        return;
      }
      sendJson(res, 200, { valid: true, payload });
    },
  },
  // GET /api/auth/me
  {
    method: 'GET',
    pattern: '/api/auth/me',
    authRequired: true,
    handler: async (req: IncomingMessage, res: ServerResponse) => {
      const authHeader = req.headers['authorization'] as string;
      const result = authenticateRequest(authHeader);
      if (!result) {
        sendJson(res, 401, { error: 'Unauthorized' });
        return;
      }
      sendJson(res, 200, {
        user: {
          id: result.user.id,
          email: result.user.email,
          name: result.user.name,
          role: result.user.role,
        },
      });
    },
  },
];
