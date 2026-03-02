use crate::adapters::{IpcServer, LogindSessionAdapter, LogindSignal, PamAuthenticator};
use crate::app::config::load_daemon_config;
use crate::app::ui_process::UiProcessManager;
use crate::domain::{AuthGuard, AuthPolicy, LockState};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

pub fn run() -> anyhow::Result<()> {
    init_tracing();
    info!("kwylock-daemon started");

    let daemon_config = load_daemon_config()?;
    let lock_state = Arc::new(Mutex::new(LockState::Unlocked));
    let (signal_tx, signal_rx) = mpsc::channel();
    let authenticator = PamAuthenticator::from_env()?;
    let mut ui_process = UiProcessManager::new(daemon_config.ui_command.clone());
    let mut auth_guard = AuthGuard::new(AuthPolicy {
        max_failures_before_lockout: daemon_config.auth.max_failures_before_lockout,
        initial_backoff_ms: daemon_config.auth.initial_backoff_ms,
        max_backoff_ms: daemon_config.auth.max_backoff_ms,
        lockout_seconds: daemon_config.auth.lockout_seconds,
    });

    let _logind_listener = LogindSessionAdapter::spawn_listener(signal_tx);
    let ipc_server = IpcServer::bind_default()?;
    info!(path = %ipc_server.socket_path().display(), "daemon IPC socket ready");
    info!(
        ui_command = %daemon_config.ui_command.join(" "),
        "daemon UI command configured"
    );
    info!("IPC server accepting clients");

    loop {
        while let Ok(signal) = signal_rx.try_recv() {
            apply_logind_signal(signal, &lock_state, &mut ui_process);
        }

        if let Err(err) = ipc_server.poll(
            &lock_state,
            &authenticator,
            &LogindSessionAdapter::unlock_current_session,
            &mut auth_guard,
        ) {
            error!(error = %err, "IPC poll failed");
        }

        thread::sleep(Duration::from_millis(30));
    }
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kwylock_daemon=info"));

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn apply_logind_signal(
    signal: LogindSignal,
    lock_state: &Arc<Mutex<LockState>>,
    ui_process: &mut UiProcessManager,
) {
    let mut state = match lock_state.lock() {
        Ok(state) => state,
        Err(err) => {
            error!(error = %err, "lock state poisoned");
            return;
        }
    };

    match signal {
        LogindSignal::SessionLocked => {
            *state = LockState::Locked;
            info!("session lock signal received");
            if let Err(err) = ui_process.ensure_running() {
                warn!(error = %err, "failed to start UI process");
            }
        }
        LogindSignal::SessionUnlocked => {
            *state = LockState::Unlocked;
            info!("session unlock signal received");
            if let Err(err) = ui_process.ensure_stopped() {
                warn!(error = %err, "failed to stop UI process");
            }
        }
        LogindSignal::PrepareForSleep(start) => {
            if start {
                *state = LockState::Locked;
                info!("prepare-for-sleep=true; forcing locked state");
                if let Err(err) = ui_process.ensure_running() {
                    warn!(error = %err, "failed to start UI before suspend");
                }
            } else {
                info!("prepare-for-sleep=false");
            }
        }
    }
}
