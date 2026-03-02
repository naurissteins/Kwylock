use anyhow::{Context, Result};
use dbus::blocking::Connection;
use dbus::message::MatchRule;
use dbus::{MessageType, Path};
use std::env;
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

const LOGIN1_DESTINATION: &str = "org.freedesktop.login1";
const LOGIN1_MANAGER_PATH: &str = "/org/freedesktop/login1";
const LOGIN1_MANAGER_INTERFACE: &str = "org.freedesktop.login1.Manager";
const LOGIN1_SESSION_INTERFACE: &str = "org.freedesktop.login1.Session";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogindSignal {
    SessionLocked,
    SessionUnlocked,
    PrepareForSleep(bool),
}

#[derive(Debug, Default)]
pub struct LogindSessionAdapter;

impl LogindSessionAdapter {
    pub fn spawn_listener(signal_tx: Sender<LogindSignal>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            loop {
                if let Err(err) = listen_blocking(signal_tx.clone()) {
                    error!(error = %err, "logind listener failed; retrying");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        })
    }

    pub fn unlock_current_session() -> Result<()> {
        let session_id = env::var("XDG_SESSION_ID")
            .ok()
            .filter(|value| !value.is_empty());

        if let Some(session_id) = &session_id {
            match unlock_via_dbus(session_id) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    warn!(
                        session_id = %session_id,
                        error = %err,
                        "UnlockSession via D-Bus failed; falling back to loginctl"
                    );
                }
            }
        } else {
            warn!(
                "XDG_SESSION_ID not set; skipping direct D-Bus unlock and using loginctl fallback"
            );
        }

        unlock_via_loginctl(session_id.as_deref())
    }
}

fn unlock_via_dbus(session_id: &str) -> Result<()> {
    let connection = Connection::new_system().context("failed connecting to system D-Bus")?;
    let manager_proxy = connection.with_proxy(
        LOGIN1_DESTINATION,
        LOGIN1_MANAGER_PATH,
        Duration::from_secs(2),
    );

    let _: () = manager_proxy
        .method_call(LOGIN1_MANAGER_INTERFACE, "UnlockSession", (session_id,))
        .with_context(|| format!("UnlockSession({session_id}) failed"))?;

    info!(session_id = %session_id, "unlock request sent to logind");
    Ok(())
}

fn unlock_via_loginctl(session_id: Option<&str>) -> Result<()> {
    let mut command = Command::new("loginctl");
    command.arg("unlock-session");

    if let Some(session_id) = session_id {
        command.arg(session_id);
    }

    let output = command
        .output()
        .context("failed executing loginctl unlock-session")?;
    if output.status.success() {
        info!("unlock request sent via loginctl fallback");
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow::anyhow!(
        "loginctl unlock-session failed with status {}: {}",
        output.status,
        stderr.trim()
    ))
}

fn listen_blocking(signal_tx: Sender<LogindSignal>) -> Result<()> {
    let connection = Connection::new_system().context("failed connecting to system D-Bus")?;
    let session_path = discover_session_path(&connection)?;

    info!(path = %session_path, "listening to logind session signals");
    subscribe_manager_prepare_for_sleep(&connection, signal_tx.clone())?;
    subscribe_session_lock_signals(&connection, session_path, signal_tx)?;

    loop {
        connection
            .process(Duration::from_millis(1_000))
            .context("D-Bus signal processing failed")?;
    }
}

fn discover_session_path(connection: &Connection) -> Result<Path<'static>> {
    let manager_proxy = connection.with_proxy(
        LOGIN1_DESTINATION,
        LOGIN1_MANAGER_PATH,
        Duration::from_secs(2),
    );

    if let Ok(session_id) = env::var("XDG_SESSION_ID")
        && !session_id.is_empty()
    {
        match manager_proxy.method_call(
            LOGIN1_MANAGER_INTERFACE,
            "GetSession",
            (session_id.clone(),),
        ) {
            Ok((session_path,)) => return Ok(session_path),
            Err(err) => {
                warn!(
                    session_id = %session_id,
                    error = %err,
                    "GetSession(XDG_SESSION_ID) failed; falling back to GetSessionByPID"
                );
            }
        }
    }

    let (session_path,): (Path<'static>,) = manager_proxy
        .method_call(
            LOGIN1_MANAGER_INTERFACE,
            "GetSessionByPID",
            (std::process::id(),),
        )
        .context("GetSessionByPID failed")?;

    Ok(session_path)
}

fn subscribe_manager_prepare_for_sleep(
    connection: &Connection,
    signal_tx: Sender<LogindSignal>,
) -> Result<()> {
    let match_rule = MatchRule::new_signal(LOGIN1_MANAGER_INTERFACE, "PrepareForSleep")
        .with_path(LOGIN1_MANAGER_PATH)
        .with_type(MessageType::Signal);

    connection
        .add_match(match_rule, move |(start,): (bool,), _, _| {
            let _ = signal_tx.send(LogindSignal::PrepareForSleep(start));
            true
        })
        .map(|_| ())
        .context("failed subscribing to PrepareForSleep signal")
}

fn subscribe_session_lock_signals(
    connection: &Connection,
    session_path: Path<'static>,
    signal_tx: Sender<LogindSignal>,
) -> Result<()> {
    let session_path = session_path.into_static();

    let lock_match = MatchRule::new_signal(LOGIN1_SESSION_INTERFACE, "Lock")
        .with_path(session_path.clone())
        .with_type(MessageType::Signal);
    let unlock_match = MatchRule::new_signal(LOGIN1_SESSION_INTERFACE, "Unlock")
        .with_path(session_path.clone())
        .with_type(MessageType::Signal);

    let lock_tx = signal_tx.clone();
    connection
        .add_match(lock_match, move |_: (), _, _| {
            let _ = lock_tx.send(LogindSignal::SessionLocked);
            true
        })
        .map(|_| ())
        .context("failed subscribing to session Lock signal")?;

    connection
        .add_match(unlock_match, move |_: (), _, _| {
            let _ = signal_tx.send(LogindSignal::SessionUnlocked);
            true
        })
        .map(|_| ())
        .context("failed subscribing to session Unlock signal")
}
