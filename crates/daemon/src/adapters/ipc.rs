use crate::adapters::LogindSignal;
use crate::domain::LockState;
use anyhow::{Context, Result};
use kwylock_common::ipc::{DaemonToUi, UiToDaemon};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct IpcServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn bind_default() -> Result<Self> {
        let socket_path = kwylock_common::ipc::socket_path();
        let parent_dir = socket_path
            .parent()
            .context("missing parent directory for IPC socket path")?;

        fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "failed creating IPC parent directory {}",
                parent_dir.display()
            )
        })?;

        remove_stale_socket(&socket_path)?;

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("failed binding IPC socket at {}", socket_path.display()))?;
        listener
            .set_nonblocking(true)
            .context("failed setting IPC socket to non-blocking mode")?;

        Ok(Self {
            listener,
            socket_path,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn run(
        &self,
        lock_state: Arc<Mutex<LockState>>,
        logind_rx: Receiver<LogindSignal>,
    ) -> Result<()> {
        info!("IPC server accepting clients");

        loop {
            drain_logind_events(&logind_rx, &lock_state);

            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    if let Err(err) = handle_client(stream, &lock_state) {
                        warn!(error = %err, "failed handling IPC client");
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(err) => {
                    return Err(err).context("IPC accept loop failed");
                }
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_file(&self.socket_path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                error = %err,
                path = %self.socket_path.display(),
                "failed to remove IPC socket on shutdown"
            );
        }
    }
}

fn remove_stale_socket(socket_path: &Path) -> Result<()> {
    match fs::remove_file(socket_path) {
        Ok(()) => {
            debug!(path = %socket_path.display(), "removed stale IPC socket");
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err)
            .with_context(|| format!("failed removing stale IPC socket {}", socket_path.display())),
    }
}

fn drain_logind_events(logind_rx: &Receiver<LogindSignal>, lock_state: &Arc<Mutex<LockState>>) {
    while let Ok(signal) = logind_rx.try_recv() {
        let mut state = match lock_state.lock() {
            Ok(state) => state,
            Err(err) => {
                error!(error = %err, "lock state poisoned");
                continue;
            }
        };

        match signal {
            LogindSignal::SessionLocked => {
                *state = LockState::Locked;
                info!("session lock signal received");
            }
            LogindSignal::SessionUnlocked => {
                *state = LockState::Unlocked;
                info!("session unlock signal received");
            }
            LogindSignal::PrepareForSleep(start) => {
                if start {
                    *state = LockState::Locked;
                    info!("prepare-for-sleep=true; lock state forced to locked");
                } else {
                    info!("prepare-for-sleep=false");
                }
            }
        }
    }
}

fn handle_client(stream: UnixStream, lock_state: &Arc<Mutex<LockState>>) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();

    reader
        .read_line(&mut request)
        .context("failed reading IPC request line")?;

    let request: UiToDaemon = serde_json::from_str(request.trim_end())
        .context("failed deserializing UI->daemon IPC message")?;

    let response = match request {
        UiToDaemon::UnlockAttempt { password } => handle_unlock_attempt(password, lock_state),
    };

    let stream = reader.get_mut();
    serde_json::to_writer(&mut *stream, &response)
        .context("failed serializing daemon->UI IPC message")?;
    stream
        .write_all(b"\n")
        .context("failed writing IPC response delimiter")?;
    stream.flush().context("failed flushing IPC response")
}

fn handle_unlock_attempt(password: String, lock_state: &Arc<Mutex<LockState>>) -> DaemonToUi {
    // Temporary prototype policy; this will be replaced with PAM.
    let accepted = password == "test";

    match lock_state.lock() {
        Ok(mut state) => {
            if accepted {
                *state = LockState::Unlocked;
                info!("unlock accepted by prototype policy");
                DaemonToUi::UnlockAccepted
            } else {
                *state = LockState::Locked;
                info!("unlock rejected by prototype policy");
                DaemonToUi::UnlockRejected
            }
        }
        Err(err) => {
            error!(error = %err, "lock state poisoned while handling unlock attempt");
            DaemonToUi::UnlockRejected
        }
    }
}
