export { signToken, verifyToken, authenticateRequest, createUser, getUserByEmail, getUserById, generateApiKey, hasRole } from './jwt';
export type { AuthUser, TokenPayload, UserRole } from './jwt';
export { requireAuth, requireRoles } from './middleware';
export type { AuthResult } from './middleware';
