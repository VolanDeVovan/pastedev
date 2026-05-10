// Mirrors paste-core::user / snippet / scope. Adjust here when the server side changes.

export type Role = 'user' | 'admin';
export type UserStatus = 'pending' | 'approved' | 'rejected' | 'suspended';
export type SnippetType = 'code' | 'markdown' | 'html';
export type Scope = 'publish' | 'read' | 'delete';

export interface UserPublic {
  id: string;
  username: string;
  role: Role;
  status: UserStatus;
  created_at: string;
}

export interface SetupCheck {
  id: string;
  status: 'ok' | 'warn' | 'err' | 'pend';
  detail: string;
}

export interface SetupStatus {
  needs_setup: boolean;
  version: string;
  checks: SetupCheck[];
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface AdminUserView {
  id: string;
  username: string;
  email: string | null;
  reason: string | null;
  registration_ip: string | null;
  status: UserStatus;
  role: Role;
  created_at: string;
}

export interface AdminUserList {
  items: AdminUserView[];
  next_cursor: string | null;
}
