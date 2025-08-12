use tokio::sync::watch::{Receiver, Sender};

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
        self.state_tx.send(new_state);
    }

    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }
}
