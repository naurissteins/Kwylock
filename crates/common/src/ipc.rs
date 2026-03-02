use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum UiToDaemon {
    UnlockAttempt { password: String },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub enum DaemonToUi {
    UnlockAccepted,
    UnlockRejected,
}

pub fn socket_path() -> PathBuf {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));

    runtime_dir.join("kwylock").join("daemon.sock")
}
