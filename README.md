# HTTPS Server Checker

A native Windows GUI application written in Rust that checks the HTTP `Server`
header of any URL.

## Architecture

| Requirement       | Crate / Tool                  |
|--------------------|-------------------------------|
| Win32 GUI          | `windows` crate (v0.58)       |
| TLS                | `rustls` (pure-Rust, no OpenSSL) |
| HTTP client        | `ureq` (uses rustls via `tls` feature) |
| Cross-compiler     | MinGW (`x86_64-w64-mingw32-gcc`) |

## Prerequisites

1. **Rust toolchain** — install via [rustup](https://rustup.rs)
2. **MinGW-w64 cross-compiler**
   - Ubuntu/Debian: `sudo apt install gcc-mingw-w64-x86-64`
   - Arch: `sudo pacman -S mingw-w64-gcc`
   - macOS: `brew install mingw-w64`
   - Windows (native): install [MSYS2](https://www.msys2.org) and its MinGW-w64 toolchain
3. **Windows GNU target for Rust**:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   ```

## Build

```bash
cargo build --target x86_64-pc-windows-gnu --release
```

The resulting `.exe` is at:
```
target/x86_64-pc-windows-gnu/release/server-checker.exe
```

## Usage

1. Run `server-checker.exe` on any Windows machine (no runtime dependencies).
2. Type a URL into the text box (e.g. `https://www.google.com`).
3. Click **Check Server**.
4. A message box pops up showing the `Server` header value,
   or **"Unknown Server"** if the header is absent.

## How It Works

- The GUI is built entirely with the **`windows`** crate's Win32 bindings
  (RegisterClassExW → CreateWindowExW → message loop).
- On button click the app reads the URL from the Edit control, sends an
  HTTPS `HEAD` request via **`ureq`** (which negotiates TLS through **`rustls`**),
  and inspects the response's `Server` header.
- The result is displayed in a standard Win32 `MessageBoxW`.
- The `.cargo/config.toml` tells Cargo to use `x86_64-w64-mingw32-gcc` as the
  linker and passes `-mwindows` so no console window appears.
