import { config } from '../config';
import type {
  AdminUserList,
  AdminUserView,
  ApiError,
  SetupStatus,
  UserPublic,
  UserStatus,
} from './types';

const api = (path: string) => `${config.apiBaseUrl}${path}`;

export class HttpError extends Error {
  status: number;
  error: ApiError;
  constructor(status: number, error: ApiError) {
    super(`${error.code}: ${error.message}`);
    this.status = status;
    this.error = error;
  }
}

async function parse<T>(r: Response): Promise<T> {
  if (r.status === 204) {
    return undefined as T;
  }
  const text = await r.text();
  if (!r.ok) {
    let envelope: { error: ApiError } | null = null;
    try {
      envelope = JSON.parse(text);
    } catch {
      // fall through
    }
    const err: ApiError = envelope?.error ?? {
      code: 'unknown',
      message: text || `HTTP ${r.status}`,
    };
    throw new HttpError(r.status, err);
  }
  return text ? (JSON.parse(text) as T) : (undefined as T);
}

async function call<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const init: RequestInit = {
    method,
    credentials: 'include',
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  };
  const r = await fetch(api(path), init);
  return parse<T>(r);
}

// setup
export const fetchSetupStatus = () => call<SetupStatus>('GET', '/api/v1/setup/status');
export const createFirstAdmin = (input: {
  username: string;
  email?: string;
  password: string;
}) => call<{ user: UserPublic }>('POST', '/api/v1/setup/admin', input);

// auth
export const me = () => call<UserPublic>('GET', '/api/v1/auth/me');
export const login = (input: { username: string; password: string }) =>
  call<{ user: UserPublic }>('POST', '/api/v1/auth/login', input);
export const register = (input: {
  username: string;
  email?: string;
  password: string;
  reason: string;
}) => call<{ user: UserPublic }>('POST', '/api/v1/auth/register', input);
export const logout = () => call<void>('POST', '/api/v1/auth/logout');

// admin
export const listUsers = (status?: UserStatus) => {
  const qs = status ? `?status=${encodeURIComponent(status)}` : '';
  return call<AdminUserList>('GET', `/api/v1/admin/users${qs}`);
};
export const approveUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/approve`);
export const rejectUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/reject`);
export const suspendUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/suspend`);
export const restoreUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/restore`);
export const promoteUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/promote`);
export const demoteUser = (id: string) =>
  call<{ user: UserPublic }>('POST', `/api/v1/admin/users/${id}/demote`);
export const resetPassword = (id: string, new_password: string) =>
  call<void>('POST', `/api/v1/admin/users/${id}/reset_password`, { new_password });

export type { AdminUserView, AdminUserList, SetupStatus, UserPublic, UserStatus };
