use std::{collections::HashSet, str::FromStr};

use ed25519_dalek::{
    Signature, SignatureError, SigningKey, VerifyingKey, ed25519::signature::SignerMut,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

pub const MSG_SIZE: usize = 32;

pub struct AdminAuthority {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    expired_msgs: HashSet<[u8; MSG_SIZE]>,
}

#[derive(Deserialize, Serialize)]
pub struct AdminCred {
    msg: [u8; MSG_SIZE],
    sig: String,
}
impl AdminCred {
    pub fn new(msg: [u8; MSG_SIZE], sig: String) -> Self {
        Self { msg, sig }
    }

    pub fn get_msg(&self) -> &[u8; MSG_SIZE] {
        &self.msg
    }

    pub fn get_sig(&self) -> Result<Signature, SignatureError> {
        Signature::from_str(&self.sig)
    }

    pub fn get_sig_str(&self) -> &str {
        &self.sig
    }
}

impl AdminAuthority {
    pub fn new() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        Self {
            signing_key,
            verifying_key,
            expired_msgs: HashSet::new(),
        }
    }

    pub fn new_token(&mut self) -> AdminCred {
        let mut msg = [0u8; 32];
        OsRng.fill_bytes(&mut msg);
        AdminCred::new(msg, self.signing_key.sign(&msg).to_string())
    }

    pub fn validate_token(&mut self, cred: AdminCred) -> bool {
        if let Ok(sig) = cred.get_sig()
            && !self.expired_msgs.contains(cred.get_msg()) {
                self.expired_msgs.insert(*cred.get_msg());
                return self
                    .verifying_key
                    .verify_strict(cred.get_msg(), &sig)
                    .is_ok();
            }
        false
    }
}
