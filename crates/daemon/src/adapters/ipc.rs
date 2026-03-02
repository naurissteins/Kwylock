use crate::adapters::Authenticator;
use crate::domain::LockState;
use anyhow::{Context, Result};
use kwylock_common::ipc::{DaemonToUi, UiToDaemon};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

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

    pub fn poll(
        &self,
        lock_state: &Arc<Mutex<LockState>>,
        authenticator: &dyn Authenticator,
        unlock_session: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        match self.listener.accept() {
            Ok((stream, _addr)) => handle_client(stream, lock_state, authenticator, unlock_session),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(err) => Err(err).context("IPC accept failed"),
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

fn handle_client(
    stream: UnixStream,
    lock_state: &Arc<Mutex<LockState>>,
    authenticator: &dyn Authenticator,
    unlock_session: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();

    reader
        .read_line(&mut request)
        .context("failed reading IPC request line")?;

    let request: UiToDaemon = serde_json::from_str(request.trim_end())
        .context("failed deserializing UI->daemon IPC message")?;

    let response = match request {
        UiToDaemon::UnlockAttempt { password } => {
            handle_unlock_attempt(password, lock_state, authenticator, unlock_session)
        }
    };

    let stream = reader.get_mut();
    serde_json::to_writer(&mut *stream, &response)
        .context("failed serializing daemon->UI IPC message")?;
    stream
        .write_all(b"\n")
        .context("failed writing IPC response delimiter")?;
    stream.flush().context("failed flushing IPC response")
}

fn handle_unlock_attempt(
    password: String,
    lock_state: &Arc<Mutex<LockState>>,
    authenticator: &dyn Authenticator,
    unlock_session: &dyn Fn() -> Result<()>,
) -> DaemonToUi {
    let currently_locked = match lock_state.lock() {
        Ok(state) => *state == LockState::Locked,
        Err(err) => {
            return DaemonToUi::UnlockFailed {
                reason: format!("internal lock state error: {err}"),
            };
        }
    };

    if !currently_locked {
        return DaemonToUi::UnlockFailed {
            reason: "session is not marked locked".to_string(),
        };
    }

    let accepted = match authenticator.verify_password(&password) {
        Ok(value) => value,
        Err(err) => {
            return DaemonToUi::UnlockFailed {
                reason: format!("authentication backend error: {err}"),
            };
        }
    };

    let mut state = match lock_state.lock() {
        Ok(state) => state,
        Err(err) => {
            return DaemonToUi::UnlockFailed {
                reason: format!("internal lock state error: {err}"),
            };
        }
    };

    if accepted {
        match unlock_session() {
            Ok(()) => {
                *state = LockState::Unlocked;
                info!(user = %authenticator.username(), "unlock accepted by PAM");
                return DaemonToUi::UnlockAccepted;
            }
            Err(err) => {
                *state = LockState::Locked;
                warn!(
                    error = %err,
                    user = %authenticator.username(),
                    "PAM accepted but session unlock request failed"
                );
                return DaemonToUi::UnlockFailed {
                    reason: "authentication succeeded but system unlock request failed".to_string(),
                };
            }
        }
    }

    *state = LockState::Locked;
    info!(user = %authenticator.username(), "unlock rejected by PAM");
    DaemonToUi::UnlockRejected
}
