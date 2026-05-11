//! Input validators shared between the registration handler and the
//! first-admin setup handler. Kept here so the rules don't drift.

use crate::error::AppError;

/// Trims, lowercases, and enforces `[a-z0-9_.-]{3,40}`.
pub fn normalize_username(raw: &str) -> Result<String, AppError> {
    let s = raw.trim().to_ascii_lowercase();
    let valid = s.len() >= 3
        && s.len() <= 40
        && s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.' || c == '-'
        });
    if !valid {
        return Err(AppError::Validation(
            "username must match [a-z0-9_.-]{3,40}".into(),
        ));
    }
    Ok(s)
}

/// Trims and applies basic sanity checks. Returns `Ok(None)` for blank input.
pub fn normalize_email(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.len() > 255 {
        return Err(AppError::Validation("email too long".into()));
    }
    if !s.contains('@') || !s.contains('.') {
        return Err(AppError::Validation("email looks invalid".into()));
    }
    Ok(Some(s.to_string()))
}

/// Length-only password policy. Argon2 handles entropy concerns.
pub fn validate_password(s: &str) -> Result<(), AppError> {
    if s.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    Ok(())
}
