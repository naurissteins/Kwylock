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

```bash
cd /home/ns/kwimy/Kwylock/Kwylock
GDK_BACKEND=wayland ./target/release/kwylock-ui
```

If the release binary does not exist yet, run:

```bash
GDK_BACKEND=wayland cargo run -p kwylock-ui
```

Daemon bootstrap (currently scaffold only):

```bash
cargo run -p kwylock-daemon --bin kwylock-daemon
```

## Workspace Layout

- `crates/ui`: GTK4 UI prototype.
- `crates/daemon`: lock orchestration daemon scaffold.
- `crates/common`: shared config and IPC types.

## Roadmap (Short)

1. Add layer-shell output surfaces.
2. Add daemon with logind lock/unlock handling.
3. Add PAM authentication integration.
4. Add weather/sysinfo/config/theming polish.
