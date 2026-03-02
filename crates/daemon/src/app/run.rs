use crate::adapters::{IpcServer, LogindSessionAdapter};
use crate::domain::LockState;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::EnvFilter;

pub fn run() -> anyhow::Result<()> {
    init_tracing();
    info!("kwylock-daemon started");

    let lock_state = Arc::new(Mutex::new(LockState::Unlocked));
    let (signal_tx, signal_rx) = mpsc::channel();

    let _logind_listener = LogindSessionAdapter::spawn_listener(signal_tx);
    let ipc_server = IpcServer::bind_default()?;
    info!(path = %ipc_server.socket_path().display(), "daemon IPC socket ready");

    ipc_server.run(lock_state, signal_rx)
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kwylock_daemon=info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
