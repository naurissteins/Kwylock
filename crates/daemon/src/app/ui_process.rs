use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::{Child, Command};
use tracing::{info, warn};

pub struct UiProcessManager {
    child: Option<Child>,
    ui_binary: PathBuf,
}

impl Default for UiProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UiProcessManager {
    pub fn new() -> Self {
        Self {
            child: None,
            ui_binary: resolve_ui_binary_path(),
        }
    }

    pub fn ensure_running(&mut self) -> Result<()> {
        self.refresh_exited_child();
        if self.child.is_some() {
            return Ok(());
        }

        let mut command = Command::new(&self.ui_binary);
        if env::var_os("GDK_BACKEND").is_none() {
            command.env("GDK_BACKEND", "wayland");
        }

        let child = command
            .spawn()
            .with_context(|| format!("failed spawning UI binary {}", self.ui_binary.display()))?;

        info!(
            pid = child.id(),
            binary = %self.ui_binary.display(),
            "UI process started"
        );
        self.child = Some(child);
        Ok(())
    }

    pub fn ensure_stopped(&mut self) -> Result<()> {
        self.refresh_exited_child();
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        let pid = child.id();
        if let Err(err) = child.kill()
            && err.kind() != std::io::ErrorKind::InvalidInput
        {
            return Err(err).with_context(|| format!("failed killing UI process {pid}"));
        }

        let _ = child.wait();
        info!(pid = pid, "UI process stopped");
        Ok(())
    }

    fn refresh_exited_child(&mut self) {
        let Some(child) = &mut self.child else {
            return;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                warn!(status = %status, "UI process exited");
                self.child = None;
            }
            Ok(None) => {}
            Err(err) => {
                warn!(error = %err, "failed checking UI process status");
            }
        }
    }
}

fn resolve_ui_binary_path() -> PathBuf {
    if let Some(path) = env::var_os("KWYLOCK_UI_BIN") {
        return PathBuf::from(path);
    }

    if let Ok(current_exe) = env::current_exe() {
        let sibling = current_exe.with_file_name("kwylock-ui");
        if sibling.exists() {
            return sibling;
        }
    }

    PathBuf::from("kwylock-ui")
}
