// ============================================================
// JWT authentication — token generation, verification, user store
// ============================================================

import jwt from 'jsonwebtoken';
import { randomUUID } from 'crypto';

const JWT_SECRET = process.env.JWT_SECRET || 'masday-workflow-dev-secret-change-in-production';
const JWT_EXPIRES_IN = process.env.JWT_EXPIRES_IN || '24h';

export type UserRole = 'admin' | 'user' | 'readonly';

export interface AuthUser {
  id: string;
  email: string;
  name: string;
  role: UserRole;
  apiKey?: string;
}

export interface TokenPayload {
  userId: string;
  email: string;
  role: UserRole;
}

/** In-memory user store (production would use database) */
const users: Map<string, AuthUser> = new Map();
const apiKeyMap: Map<string, string> = new Map(); // apiKey -> userId

/** Create a new user */
export function createUser(input: { email: string; name: string; role?: UserRole }): AuthUser {
  const id = `user_${randomUUID()}`;
  const user: AuthUser = {
    id,
    email: input.email,
    name: input.name,
    role: input.role || 'user',
  };
  users.set(id, user);
  return user;
}

/** Get user by email */
export function getUserByEmail(email: string): AuthUser | undefined {
  for (const user of users.values()) {
    if (user.email === email) return user;
  }
  return undefined;
}

/** Get user by ID */
export function getUserById(id: string): AuthUser | undefined {
  return users.get(id);
}

/** Generate API key for a user */
export function generateApiKey(userId: string): string {
  const user = users.get(userId);
  if (!user) throw new Error('User not found');

  const apiKey = `mw_${randomUUID().replace(/-/g, '')}`;
  user.apiKey = apiKey;
  apiKeyMap.set(apiKey, userId);
  return apiKey;
}

/** Sign a JWT token */
export function signToken(payload: TokenPayload): string {
  return jwt.sign(payload, JWT_SECRET, { expiresIn: JWT_EXPIRES_IN as jwt.SignOptions['expiresIn'] });
}

/** Verify a JWT token */
export function verifyToken(token: string): TokenPayload | null {
  try {
    return jwt.verify(token, JWT_SECRET) as TokenPayload;
  } catch {
    return null;
  }
}

/** Extract and verify user from Authorization header */
export function authenticateRequest(
  authHeader: string | undefined,
): { user: AuthUser; payload: TokenPayload } | null {
  if (!authHeader) return null;

  // Bearer token
  if (authHeader.startsWith('Bearer ')) {
    const token = authHeader.slice(7);
    const payload = verifyToken(token);
    if (!payload) return null;

    const user = getUserById(payload.userId);
    if (!user) return null;

    return { user, payload };
  }

  // API key
  if (authHeader.startsWith('ApiKey ')) {
    const apiKey = authHeader.slice(7);
    const userId = apiKeyMap.get(apiKey);
    if (!userId) return null;

    const user = users.get(userId);
    if (!user) return null;

    return {
      user,
      payload: { userId: user.id, email: user.email, role: user.role },
    };
  }

  return null;
}

/** Check if a user has one of the required roles */
export function hasRole(user: AuthUser, roles: string[]): boolean {
  if (roles.length === 0) return true;
  return roles.includes(user.role);
}
