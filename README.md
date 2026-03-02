# Kwylock

Kwylock is a Wayland-first lockscreen project focused on Hyprland.

Current status: early prototype.

## What It Is

- Rust + GTK4 lockscreen project.
- Target behavior is instant lock semantics with modern UI.
- Multi-monitor and layer-shell support are planned as core features.

## Current Prototype Status

The current binary (`kwylock-ui`) is a fullscreen GTK prototype for UI testing.

It is **not yet a secure production lockscreen** because full lock orchestration
(daemon + logind + curtain + PAM flow) is still under implementation.

What is implemented today:
- Daemon Unix-socket IPC (`$XDG_RUNTIME_DIR/kwylock/daemon.sock`)
- UI unlock requests sent to daemon
- Daemon-controlled PAM unlock accept/reject response
- Daemon requests `logind` session unlock on successful PAM auth
- logind signal subscription for `Lock`, `Unlock`, and `PrepareForSleep`
- Daemon spawns UI on lock signal and tears it down on unlock signal
- Auth hardening: retry backoff and temporary lockout on repeated failures

## Requirements (Arch Linux)

```bash
sudo pacman -S --needed rust gtk4
```

## Build

```bash
cd /home/ns/kwimy/Kwylock/Kwylock
cargo build --release --workspace
```

## Run Prototype

Start daemon first:

```bash
cd /home/ns/kwimy/Kwylock/Kwylock
cargo run -p kwylock-daemon --bin kwylock-daemon
```

Trigger lock (daemon will spawn UI automatically):

```bash
loginctl lock-session
```

Unlock with your normal Linux account password (PAM `login` service).

Manual UI run is still available for debugging:

```bash
GDK_BACKEND=wayland cargo run -p kwylock-ui
```

## Configuration

Default path:
- `$XDG_CONFIG_HOME/kwylock/config.toml`
- Fallback: `~/.config/kwylock/config.toml`

Example:

```toml
[daemon]
ui_command = ["/home/ns/kwimy/Kwylock/Kwylock/target/debug/kwylock-ui"]

[daemon.auth]
max_failures_before_lockout = 5
initial_backoff_ms = 1000
max_backoff_ms = 30000
lockout_seconds = 60
```

Optional override for config path:

```bash
KWYLOCK_CONFIG=/path/to/config.toml cargo run -p kwylock-daemon --bin kwylock-daemon
```

## Workspace Layout

- `crates/ui`: GTK4 UI prototype.
- `crates/daemon`: lock orchestration daemon + IPC server + logind listener.
- `crates/common`: shared config and IPC types.

## Roadmap (Short)

1. Add layer-shell output surfaces.
2. Add instant non-GTK curtain stage.
3. Add widget/config/theming polish.
4. Harden auth UX (retry backoff and failure feedback).
