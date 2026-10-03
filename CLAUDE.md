# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`wl-crosshair` is a small Rust CLI that draws a static crosshair/cursor image overlay on wlroots-based Wayland compositors (e.g. sway), using the `wlr-layer-shell` protocol. It's an extremely stripped-down fork of [crossover](https://github.com/lacymorrow/crossover). The entire implementation lives in a single file: `src/main.rs`.

## Build / run / dev commands

This project can be built either with plain Cargo or via the Nix flake (`use_flake` in `.envrc`, so `direnv` will auto-enter the dev shell).

```sh
cargo build --release
cargo run -- --size 24 --offset-y 100 ./dot.png
cargo test        # unit tests for the image/frame rendering
cargo fmt
```

Wayland behaviour itself is not covered by tests – validate by running against a live compositor.

Nix:
```sh
nix build      # builds the package (output at ./result/bin/wl-crosshair)
nix run        # build + run
nix develop    # enter dev shell with cargo/rustc/rustfmt
```

The Nix package wraps the binary with `WL_CROSSHAIR_IMAGE_PATH` set to the bundled `cursors/inverse-v.png`.

## Runtime requirements

Must run inside a Wayland session on a compositor implementing `wlr-layer-shell-unstable-v1` (wlroots compositors, KDE Plasma/KWin; not GNOME). No screen size is needed: the layer surface is not anchored, so the compositor centers it on the output.

## Architecture

Everything lives in `src/main.rs`:

1. **`run()`** – all errors are `Result<_, String>` and end up as one `wl-crosshair: <message>` line with exit code 1. No panics for user errors.
2. **Config + args** – `Config` (all `Option`, `#[serde(deny_unknown_fields)]`) is both the TOML shape and the CLI collector. `parse_cli_args` is hand-rolled (no `clap`), accepts `--flag value` and `--flag=value`. `resolve_settings` merges CLI over file over defaults into `Settings`, including the image-path fallback chain. `screen_width`/`screen_height` are still accepted for old configs but ignored (with a note on stderr).
3. **`render()`** – loads the image (optional Lanczos3 resize to `size`), then places it in a transparent frame of size `image + 2·|offset|` per axis, so the frame center is the screen center and the image sits `offset` away from it. Pixels are premultiplied ARGB8888, little endian. Done before connecting to Wayland so image errors fail early.
4. **Wayland setup** – one `roundtrip` to collect globals (`wl_compositor`, `wl_shm`, `zwlr_layer_shell_v1`, bound with `min(advertised, supported)` version), then: shm buffer from a `tempfile`, layer surface on `Layer::Overlay` **without anchor** (= centered) and **exclusive zone -1** (ignore panels → true screen center), empty input region (click-through), no keyboard interactivity.
5. **Main loop** – `blocking_dispatch` until the layer surface gets `Closed`. On `Configure`: ack, attach buffer, commit.

Debug output goes through the `debug!` macro and only prints with `--verbose`. Objects whose events are irrelevant use `impl_dispatch_log!`.

## Conventions

- User-facing errors: return `Err(String)` with a clear English message, never `panic!`/`expect` on user input.
- New Wayland objects without real event handling go into the `impl_dispatch_log!` list.
- Keep it a single small file; avoid heavy dependencies.
