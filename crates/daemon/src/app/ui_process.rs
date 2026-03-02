use anyhow::{Context, Result};
use std::env;
use std::process::{Child, Command};
use tracing::{info, warn};

pub struct UiProcessManager {
    child: Option<Child>,
    ui_command: Vec<String>,
}

impl Default for UiProcessManager {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl UiProcessManager {
    pub fn new(config_command: Vec<String>) -> Self {
        Self {
            child: None,
            ui_command: resolve_ui_command(config_command),
        }
    }

    pub fn ensure_running(&mut self) -> Result<()> {
        self.refresh_exited_child();
        if self.child.is_some() {
            return Ok(());
        }

        let (program, args) = self
            .ui_command
            .split_first()
            .context("UI command is empty; set daemon.ui_command in config")?;

        let mut command = Command::new(program);
        command.args(args);
        if env::var_os("GDK_BACKEND").is_none() {
            command.env("GDK_BACKEND", "wayland");
        }

        let child = command.spawn().with_context(|| {
            format!(
                "failed spawning UI command {}",
                format_ui_command(&self.ui_command)
            )
        })?;

        info!(
            pid = child.id(),
            command = %format_ui_command(&self.ui_command),
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

fn resolve_ui_command(config_command: Vec<String>) -> Vec<String> {
    if let Some(path) = env::var_os("KWYLOCK_UI_BIN") {
        return vec![path.to_string_lossy().into_owned()];
    }

    if !config_command.is_empty() {
        return config_command;
    }

    vec!["kwylock-ui".to_string()]
}

fn format_ui_command(command: &[String]) -> String {
    command.join(" ")
}
