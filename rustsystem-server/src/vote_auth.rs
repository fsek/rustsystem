use std::collections::HashSet;
use tokio::sync::watch::{Receiver, Sender};
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use rustsystem_proof::{BallotMetaData, Provider, Sha256Provider};

use crate::UUID;

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

pub type Header = Vec<u8>;

pub struct VoteRound {
    metadata: BallotMetaData,
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUID>,
    expired_signatures: HashSet<[u8; 80]>,
}
impl VoteRound {
    pub fn keys(&self) -> &AuthenticationKeys {
        &self.keys
    }

    pub fn metadata(&self) -> BallotMetaData {
        self.metadata
    }

    pub fn register_user(&mut self, uuid: UUID) {
        self.registered_voters.insert(uuid);
    }

    /// Checks if a user has already registered for voting
    pub fn is_registered(&self, uuid: UUID) -> bool {
        self.registered_voters.contains(&uuid)
    }

    pub fn is_used(&self, signature: &BlindSignature<BbsBls12381Sha256>) -> bool {
        self.expired_signatures.contains(&signature.to_bytes())
    }

    pub fn set_signature_expired(&mut self, signature: &BlindSignature<BbsBls12381Sha256>) {
        self.expired_signatures.insert(signature.to_bytes());
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
}

pub struct VoteAuthority {
    state_tx: Sender<bool>,
    round: Option<VoteRound>,
}
impl VoteAuthority {
    /// For new meeting
    pub fn new() -> Self {
        Self {
            state_tx: Sender::new(false),
            round: None,
        }
    }

    pub fn is_active(&self) -> bool {
        *self.state_tx.borrow()
    }

    pub fn start_round(&mut self, metadata: BallotMetaData, header: String) {
        let keys = Sha256Provider::generate_authentication_keys();
        let header = header.as_bytes().to_vec();
        let registered_voters = HashSet::new();
        let expired_signatures = HashSet::new();
        self.state_tx.send(true);
        self.round = Some(VoteRound {
            keys,
            header,
            registered_voters,
            expired_signatures,
            metadata,
        });
    }

    pub fn round(&mut self) -> Option<&mut VoteRound> {
        self.round.as_mut()
    }

    // This is the function that should later handle the tallying of votes
    pub fn finalize_round(&mut self) {
        self.state_tx.send(false);
        self.round = None;
    }

    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }

    // /// Resets VoteAuth for new voting round. Old ballots are no longer valid since the
    // /// keys have changed.
    // /// Voters can now re-register.
    // pub fn reset(&mut self) {
    //     self.keys = Sha256Provider::generate_authentication_keys();
    //     self.registered_voters.clear();
    //     self.expired_signatures.clear();
    // }
}
