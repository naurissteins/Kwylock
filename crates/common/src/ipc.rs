use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum UiToDaemon {
    UnlockAttempt { password: String },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum DaemonToUi {
    UnlockAccepted,
    UnlockRejected,
}
