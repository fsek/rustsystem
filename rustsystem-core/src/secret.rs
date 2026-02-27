use rand::Rng;

/// Generates a fresh 32-byte random secret. Call once at startup and store the
/// result in `AppState`. The secret is intentionally not persisted — all meeting
/// state is in-memory, so a restart already invalidates every active session.
pub fn generate_secret() -> [u8; 32] {
    let mut res = [0u8; 32];
    rand::rng().fill(&mut res);
    res
}
