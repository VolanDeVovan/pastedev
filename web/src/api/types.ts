// Mirrors pastedev-core::user / snippet / scope. Adjust here when the server side changes.

export type Role = 'user' | 'admin';
export type UserStatus = 'pending' | 'approved' | 'rejected' | 'suspended';
export type SnippetType = 'code' | 'markdown' | 'html';
export type Visibility = 'public' | 'private';
export type Scope = 'publish' | 'read' | 'delete';

/// How long a `burn_after_read` snippet stays readable after the first non-owner
/// view. Mirrors pastedev_core::BURN_AFTER_READ_WINDOW_SECONDS — used by the
/// view countdown to show "~15 min" before the first view stamps an expiry.
export const BURN_AFTER_READ_WINDOW_SECONDS = 15 * 60;

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
  // The server omits these once setup is complete to avoid fingerprinting the
  // deploy to anonymous callers. The pre-setup wizard always receives them.
  version?: string;
  checks?: SetupCheck[];
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
