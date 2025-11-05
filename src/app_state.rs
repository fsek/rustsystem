use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod meeting;

use axum::http::{Uri, uri};
use uuid::Uuid;

pub use crate::app_state::meeting::*;

#[derive(Debug, Clone)]
pub struct AppState {
    pub meetings: Meetings,
    pub public_uri: Arc<Uri>,
}

impl AppState {
    pub fn new(public_url: Uri) -> Self {
        Self {
            meetings: Default::default(),
            public_uri: Arc::new(public_url),
        }
    }

    pub fn ws_base_uri(&self) -> uri::Builder {
        let scheme = match self.public_uri.scheme_str() {
            Some("https") => "wss",
            _ => "ws",
        };

        Uri::builder()
            .scheme(scheme)
            .authority(self.public_uri.authority().unwrap().clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Meetings {
    inner: Arc<Mutex<HashMap<Uuid, Meeting>>>,
}

impl Meetings {
    pub fn create(&self) -> (Uuid, Meeting) {
        let meeting_id = Uuid::new_v4();
        let meeting = Meeting::new();

        let mut meetings = self.inner.lock().unwrap();
        meetings.insert(meeting_id, meeting.clone());

        (meeting_id, meeting)
    }

    pub fn get(&self, id: &Uuid) -> Option<Meeting> {
        let meetings = self.inner.lock().unwrap();
        meetings.get(id).cloned()
    }
}
