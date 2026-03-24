# Contributing to RustBrush

Thanks for your interest in contributing! This guide covers how to build, test, and submit changes.

## Building from Source

```bash
git clone https://github.com/hopdad/RustBrush.git
cd RustBrush
cargo build --release
```

No extra dependencies needed. RustBrush targets Windows only.

## Running Tests

```bash
cargo test --workspace
```

Tests run against `rustbrush-core` (the pure logic crate). Platform-specific code in `rustbrush-platform` is not unit-tested since it requires a display server.

## Project Structure

RustBrush is a three-crate Cargo workspace:

| Crate | Role |
|-------|------|
| `rustbrush-core` | Pure library: color science, image processing, paint planning, canvas presets, text rendering, clipart, session management |
| `rustbrush-platform` | OS-specific: input simulation (enigo), screen capture, hotkeys, paint executor |
| `rustbrush-app` | Application: CLI (`main.rs`) and GUI (`gui.rs`) |

**Key design principle:** Core logic has no OS dependencies and is fully testable on any platform. All OS interaction is isolated in the platform crate.

## Code Style

- Format with `cargo fmt` before committing
- Run `cargo clippy --workspace` and fix any warnings
- Both are enforced by CI

## CI

CI runs on **Windows** and checks:
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- Release build (`cargo build --release`)

Make sure your changes pass `cargo test` and `cargo clippy` locally before opening a PR.

## Submitting Changes

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Run `cargo fmt`, `cargo clippy`, and `cargo test`
5. Open a pull request with a clear description of what you changed and why

## Releases

Releases are built automatically by CI when a version tag is pushed. The release artifact is a zip containing `rustbrush.exe` and `rustbrush-gui.exe` for Windows x86_64.
