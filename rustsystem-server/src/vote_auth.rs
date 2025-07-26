use std::collections::HashSet;
use tokio::sync::watch::{Receiver, Sender};
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use rustsystem_proof::{Provider, Sha256Provider};

use crate::UUID;

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

pub type Header = Vec<u8>;

pub struct VoteAuthority {
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUID>,
    expired_signatures: HashSet<[u8; 80]>,
    state_tx: Sender<bool>,
}
impl VoteAuthority {
    /// For new meeting
    pub fn new(header: String) -> Self {
        let keys = Sha256Provider::generate_authentication_keys();
        let header = header.as_bytes().to_vec();
        let registered_voters = HashSet::new();
        let expired_signatures = HashSet::new();

        Self {
            keys,
            header,
            registered_voters,
            expired_signatures,
            state_tx: Sender::new(false),
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn keys(&self) -> &AuthenticationKeys {
        &self.keys
    }

    pub fn is_active(&self) -> bool {
        *self.state_tx.borrow()
    }
    pub fn set_active_state(&mut self, new_state: bool) {
        self.state_tx.send(new_state);
    }
    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }

    /// Resets VoteAuth for new voting round. Old ballots are no longer valid since the
    /// keys have changed.
    /// Voters can now re-register.
    pub fn reset(&mut self) {
        self.keys = Sha256Provider::generate_authentication_keys();
        self.registered_voters.clear();
        self.expired_signatures.clear();
    }

    /// Checks if a user has already registered for voting
    pub fn is_registered(&self, uuid: UUID) -> bool {
        self.registered_voters.contains(&uuid)
    }

    pub fn register_user(&mut self, uuid: UUID) {
        self.registered_voters.insert(uuid);
    }

    pub fn is_used(&self, signature: &BlindSignature<BbsBls12381Sha256>) -> bool {
        self.expired_signatures.contains(&signature.to_bytes())
    }

    pub fn set_signature_expired(&mut self, signature: &BlindSignature<BbsBls12381Sha256>) {
        self.expired_signatures.insert(signature.to_bytes());
    }
}
