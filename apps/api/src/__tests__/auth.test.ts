// ============================================================
// Tests for JWT authentication
// ============================================================

import { describe, it, expect, beforeEach } from 'vitest';
import {
  createUser,
  getUserByEmail,
  getUserById,
  signToken,
  verifyToken,
  authenticateRequest,
  generateApiKey,
  hasRole,
} from '../auth/jwt';
import type { AuthUser } from '../auth/jwt';
import { requireAuth, requireRoles } from '../auth/middleware';
import { APIError } from '../utils';

describe('JWT Authentication', () => {
  beforeEach(() => {
    // The user store is module-level; we test by creating users fresh
  });

  describe('createUser', () => {
    it('should create a user with default role', () => {
      const user = createUser({ email: 'test@example.com', name: 'Test User' });
      expect(user.email).toBe('test@example.com');
      expect(user.name).toBe('Test User');
      expect(user.role).toBe('user');
      expect(user.id).toMatch(/^user_/);
    });

    it('should create a user with specified role', () => {
      const user = createUser({ email: 'admin@example.com', name: 'Admin', role: 'admin' });
      expect(user.role).toBe('admin');
    });

    it('should create a readonly user', () => {
      const user = createUser({ email: 'viewer@example.com', name: 'Viewer', role: 'readonly' });
      expect(user.role).toBe('readonly');
    });
  });

  describe('getUserByEmail', () => {
    it('should find a user by email', () => {
      const created = createUser({ email: 'find@example.com', name: 'Find Me' });
      const found = getUserByEmail('find@example.com');
      expect(found).toBeDefined();
      expect(found!.id).toBe(created.id);
    });

    it('should return undefined for unknown email', () => {
      const found = getUserByEmail('nonexistent@example.com');
      expect(found).toBeUndefined();
    });
  });

  describe('getUserById', () => {
    it('should find a user by ID', () => {
      const created = createUser({ email: 'byid@example.com', name: 'ById' });
      const found = getUserById(created.id);
      expect(found).toBeDefined();
      expect(found!.email).toBe('byid@example.com');
    });

    it('should return undefined for unknown ID', () => {
      const found = getUserById('user_nonexistent');
      expect(found).toBeUndefined();
    });
  });

  describe('signToken / verifyToken', () => {
    it('should sign and verify a token', () => {
      const payload = { userId: 'user_123', email: 'test@example.com', role: 'user' as const };
      const token = signToken(payload);
      expect(token).toBeTruthy();

      const verified = verifyToken(token);
      expect(verified).toBeTruthy();
      expect(verified!.userId).toBe('user_123');
      expect(verified!.email).toBe('test@example.com');
      expect(verified!.role).toBe('user');
    });

    it('should return null for invalid token', () => {
      const result = verifyToken('invalid-token');
      expect(result).toBeNull();
    });

    it('should return null for empty token', () => {
      const result = verifyToken('');
      expect(result).toBeNull();
    });
  });

  describe('authenticateRequest', () => {
    it('should authenticate a valid Bearer token', () => {
      const user = createUser({ email: 'bearer@example.com', name: 'Bearer' });
      const token = signToken({ userId: user.id, email: user.email, role: user.role });

      const result = authenticateRequest(`Bearer ${token}`);
      expect(result).toBeTruthy();
      expect(result!.user.id).toBe(user.id);
      expect(result!.payload.userId).toBe(user.id);
    });

    it('should reject invalid Bearer token', () => {
      const result = authenticateRequest('Bearer invalid-token');
      expect(result).toBeNull();
    });

    it('should return null for missing header', () => {
      const result = authenticateRequest(undefined);
      expect(result).toBeNull();
    });

    it('should return null for empty header', () => {
      const result = authenticateRequest('');
      expect(result).toBeNull();
    });
  });

  describe('generateApiKey', () => {
    it('should generate a valid API key', () => {
      const user = createUser({ email: 'apikey@example.com', name: 'ApiKey' });
      const key = generateApiKey(user.id);
      expect(key).toMatch(/^mw_/);
    });

    it('should authenticate via API key', () => {
      const user = createUser({ email: 'apikey-auth@example.com', name: 'ApiKeyAuth' });
      const key = generateApiKey(user.id);

      const result = authenticateRequest(`ApiKey ${key}`);
      expect(result).toBeTruthy();
      expect(result!.user.id).toBe(user.id);
    });

    it('should throw for nonexistent user', () => {
      expect(() => generateApiKey('user_nonexistent')).toThrow('User not found');
    });
  });

  describe('hasRole', () => {
    it('should return true when no roles required', () => {
      const user = createUser({ email: 'norole@example.com', name: 'NoRole' });
      expect(hasRole(user, [])).toBe(true);
    });

    it('should return true when user has required role', () => {
      const user = createUser({ email: 'hasrole@example.com', name: 'HasRole', role: 'admin' });
      expect(hasRole(user, ['admin'])).toBe(true);
    });

    it('should return false when user lacks required role', () => {
      const user = createUser({ email: 'norole2@example.com', name: 'NoRole2', role: 'readonly' });
      expect(hasRole(user, ['admin'])).toBe(false);
    });

    it('should return true when user has one of multiple roles', () => {
      const user = createUser({ email: 'multi@example.com', name: 'Multi', role: 'user' });
      expect(hasRole(user, ['admin', 'user'])).toBe(true);
    });
  });

  describe('requireAuth', () => {
    it('should throw for missing authorization header', () => {
      const req = { headers: {} } as any;
      expect(() => requireAuth(req)).toThrow(APIError);
      try {
        requireAuth(req);
      } catch (err) {
        expect((err as APIError).statusCode).toBe(401);
      }
    });

    it('should throw for invalid token', () => {
      const req = { headers: { authorization: 'Bearer invalid' } } as any;
      expect(() => requireAuth(req)).toThrow(APIError);
    });

    it('should return auth result for valid token', () => {
      const user = createUser({ email: 'reqauth@example.com', name: 'ReqAuth' });
      const token = signToken({ userId: user.id, email: user.email, role: user.role });
      const req = { headers: { authorization: `Bearer ${token}` } } as any;

      const result = requireAuth(req);
      expect(result.user.id).toBe(user.id);
    });
  });

  describe('requireRoles', () => {
    it('should throw 403 when user lacks role', () => {
      const user = createUser({ email: 'forbidden@example.com', name: 'Forbidden', role: 'readonly' });
      const token = signToken({ userId: user.id, email: user.email, role: user.role });
      const req = { headers: { authorization: `Bearer ${token}` } } as any;

      expect(() => requireRoles(req, ['admin'])).toThrow();
      try {
        requireRoles(req, ['admin']);
      } catch (err) {
        expect((err as APIError).statusCode).toBe(403);
      }
    });
  });
});
