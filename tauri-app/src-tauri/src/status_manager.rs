use std::sync::{Arc, Mutex};

use weibosdk_rs::session::Session;

#[derive(Debug, Default)]
pub struct StatusManager {
    session: Mutex<Option<Arc<Mutex<Session>>>>,
}

impl StatusManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_session(&self, session: Arc<Mutex<Session>>) {
        *self.session.lock().unwrap() = Some(session);
    }

    pub fn session(&self) -> Option<Arc<Mutex<Session>>> {
        self.session.lock().unwrap().clone()
    }
}
