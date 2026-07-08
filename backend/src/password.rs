use bcrypt::{hash, verify};

/// Cost 10 matches Spring's BCryptPasswordEncoder default so newly created
/// hashes take about the same time to verify as pre-existing ones.
const BCRYPT_COST: u32 = 10;

pub fn hash_password(raw: &str) -> anyhow::Result<String> {
    Ok(hash(raw, BCRYPT_COST)?)
}

pub fn verify_password(raw: &str, hashed: &str) -> bool {
    verify(raw, hashed).unwrap_or(false)
}
