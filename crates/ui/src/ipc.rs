use kwylock_common::ipc::{DaemonToUi, UiToDaemon};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub enum UnlockResult {
    Accepted,
    Rejected,
    TransportError(String),
}

pub fn unlock_attempt(password: String) -> UnlockResult {
    let socket_path = kwylock_common::ipc::socket_path();

    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(stream) => stream,
        Err(err) => {
            return UnlockResult::TransportError(format!(
                "unable to connect daemon IPC {}: {err}",
                socket_path.display()
            ));
        }
    };

    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let request = UiToDaemon::UnlockAttempt { password };
    if let Err(err) = serde_json::to_writer(&mut stream, &request) {
        return UnlockResult::TransportError(format!("unable to serialize request: {err}"));
    }

    if let Err(err) = stream.write_all(b"\n") {
        return UnlockResult::TransportError(format!("unable to write request delimiter: {err}"));
    }

    if let Err(err) = stream.flush() {
        return UnlockResult::TransportError(format!("unable to flush request: {err}"));
    }

    let mut response = String::new();
    let mut reader = BufReader::new(stream);
    if let Err(err) = reader.read_line(&mut response) {
        return UnlockResult::TransportError(format!("unable to read response: {err}"));
    }

    match serde_json::from_str::<DaemonToUi>(response.trim_end()) {
        Ok(DaemonToUi::UnlockAccepted) => UnlockResult::Accepted,
        Ok(DaemonToUi::UnlockRejected) => UnlockResult::Rejected,
        Err(err) => UnlockResult::TransportError(format!("unable to decode response: {err}")),
    }
}
