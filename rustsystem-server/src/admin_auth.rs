use std::collections::HashSet;

use rand_core::{OsRng, RngCore};

pub struct AdminAuthority {
    pending: HashSet<[u8; 16]>,
}

impl AdminAuthority {
    pub fn new() -> Self {
        Self {
            pending: HashSet::new(),
        }
    }

    /// Generate a new one-time admin token. Returns 16 random bytes that must
    /// be redeemed exactly once via [`redeem_token`].
    pub fn new_token(&mut self) -> [u8; 16] {
        let mut token = [0u8; 16];
        OsRng.fill_bytes(&mut token);
        self.pending.insert(token);
        token
    }

    /// Consume a token. Returns `true` if the token was present (and removes
    /// it), `false` if it was never issued or already redeemed.
    pub fn redeem_token(&mut self, token: [u8; 16]) -> bool {
        self.pending.remove(&token)
    }
}
