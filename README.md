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
- Daemon-controlled unlock accept/reject response
- logind signal subscription for `Lock`, `Unlock`, and `PrepareForSleep`

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

In another terminal, start UI:

```bash
cd /home/ns/kwimy/Kwylock/Kwylock
GDK_BACKEND=wayland ./target/release/kwylock-ui
```

If the release binary does not exist yet, run:

```bash
GDK_BACKEND=wayland cargo run -p kwylock-ui
```

Prototype unlock password for current test build: `test`

## Workspace Layout

- `crates/ui`: GTK4 UI prototype.
- `crates/daemon`: lock orchestration daemon + IPC server + logind listener.
- `crates/common`: shared config and IPC types.

## Roadmap (Short)

1. Add layer-shell output surfaces.
2. Add daemon with logind lock/unlock handling.
3. Add PAM authentication integration.
4. Add weather/sysinfo/config/theming polish.
