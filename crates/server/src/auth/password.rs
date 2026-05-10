//! Argon2id password hashing. Parameters target ~150ms on production hardware
//! and are tunable via env (`ARGON2_M_KIB`, `ARGON2_T_COST`).

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("invalid argon2 parameters: {0}")]
    Params(String),
    #[error("hashing failed: {0}")]
    Hash(String),
}

fn argon2(m_kib: u32, t_cost: u32) -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(m_kib, t_cost, 1, None)
        .map_err(|e| PasswordError::Params(e.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn hash(plain: &str, m_kib: u32, t_cost: u32) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = argon2(m_kib, t_cost)?;
    let phc = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| PasswordError::Hash(e.to_string()))?;
    Ok(phc.to_string())
}

/// Returns `true` on a verified password, `false` otherwise. A malformed PHC
/// string is treated as "doesn't match" — never panics.
pub fn verify(plain: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}

/// Constant-time dummy verify used when the username didn't exist — ensures
/// timing doesn't leak which side of the login pair was wrong. The dummy PHC
/// is computed once on first call and cached; subsequent calls only pay the
/// verify cost (same as the real path).
pub fn dummy_verify(m_kib: u32, t_cost: u32) {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    let phc = DUMMY.get_or_init(|| {
        hash("dummy-password-for-timing", m_kib, t_cost)
            .expect("hashing the dummy password must succeed")
    });
    let _ = verify("dummy-password-for-timing-wrong", phc);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrips() {
        // Lower the cost so the test runs fast.
        let phc = hash("correct horse battery staple", 4_096, 1).expect("hash");
        assert!(verify("correct horse battery staple", &phc));
        assert!(!verify("wrong", &phc));
    }
}
