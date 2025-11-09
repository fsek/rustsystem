use tokio::sync::watch::{Receiver, Sender};
use tracing::error;

pub struct InviteAuthority {
    state_tx: Sender<bool>,
}
impl InviteAuthority {
    pub fn new() -> Self {
        Self {
            state_tx: Sender::new(false),
        }
    }

    pub fn set_state(&mut self, new_state: bool) {
        if let Err(e) = self.state_tx.send(new_state) {
            error!("{e}");
        }
    }

    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }
}
