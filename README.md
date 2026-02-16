# HolePatch GUI

HolePatch GUI is a desktop client for NAT hole punching configuration and
runtime management. The app is built with Rust and GPUI.

## Project Structure

- `src/`: application source code
- `Cargo.toml`: Rust manifest
- `installer/`: Inno Setup script for Windows installer
- `Cargo.lock`: dependency lockfile

## Build

### Requirements

- Rust toolchain (stable)
- For Windows packaging: Inno Setup 6 (`iscc` command)

### Build binary

```bash
cargo build --release
```

Build output location:

- Windows: `target/x86_64-pc-windows-msvc/release/holepatch-gui.exe`
- macOS/Linux: `target/release/holepatch-gui`

### Run

```bash
cargo run --release
```

## Create a Windows Installer

From a machine with Inno Setup installed:

```bash
iscc installer/holepatch-gui.iss
```

The generated installer will be placed in `dist/`.

If `version` in `Cargo.toml` changes, update `MyAppVersion` in
`installer/holepatch-gui.iss` for installer filename/version consistency.

## Notes

- The window title is configured in `src/gui/app.rs`.
- This repository does not include signing or update server configuration yet.
