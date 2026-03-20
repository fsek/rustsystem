use tokio::sync::watch::{Receiver, Sender};
use tracing::error;

pub struct InviteAuthority {
    state_tx: Sender<Option<String>>,
}
impl InviteAuthority {
    pub fn new() -> Self {
        Self {
            state_tx: Sender::new(None),
        }
    }

    pub fn notify_login(&mut self, voter_name: String) {
        if let Err(e) = self.state_tx.send(Some(voter_name)) {
            error!("{e}");
        }
    }

    pub fn new_watcher(&self) -> Receiver<Option<String>> {
        self.state_tx.subscribe()
    }
}
