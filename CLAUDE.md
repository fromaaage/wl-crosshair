# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`wl-crosshair` is a small Rust CLI that draws a static crosshair/cursor image overlay on wlroots-based Wayland compositors (e.g. sway), using the `wlr-layer-shell` protocol. It's an extremely stripped-down fork of [crossover](https://github.com/lacymorrow/crossover). The entire implementation lives in a single file: `src/main.rs`.

## Build / run / dev commands

This project can be built either with plain Cargo or via the Nix flake (`use_flake` in `.envrc`, so `direnv` will auto-enter the dev shell).

```sh
cargo build --release
cargo run -- --screen-width 1920 --screen-height 1080 ./dot.png
cargo fmt
```

There is no test suite. There is no `cargo clippy`/CI config in this repo — validate changes by building and running against a live Wayland compositor.

Nix:
```sh
nix build      # builds the package (output at ./result/bin/wl-crosshair)
nix run        # build + run
nix develop    # enter dev shell with cargo/rustc/rustfmt
```

The Nix package wraps the binary with `WL_CROSSHAIR_IMAGE_PATH` set to the bundled `cursors/inverse-v.png`, so the default image is only guaranteed to resolve when run through the Nix app/package.

## Runtime requirements

Must be run inside a Wayland session on a compositor implementing `wlr-layer-shell-unstable-v1`. Originally assumed to be wlroots-only (sway, Hyprland, ...) and not to work on GNOME/KDE — but confirmed working on at least one KDE Plasma/KWin Wayland session, so treat that as compositor/version-dependent rather than a hard KDE exclusion. `screen_width`/`screen_height` are required (config file or `--screen-width`/`--screen-height`) — there's no protocol call to auto-detect output size, so the caller must supply it (e.g. from `swaymsg`, or the resolution shown in system display settings).

## Architecture

Everything happens in `src/main.rs` via a single `State` struct and the `wayland-client` `Dispatch` pattern:

1. **Config + arg parsing** — settings (`image_path`, `offset_x/y`, `size`, `screen_width/height`) can come from a TOML config file and/or CLI flags; CLI always wins. `parse_cli_args` is a hand-rolled parser (no `clap`) that fills a `Config` struct (all fields `Option`, `#[derive(Deserialize)]` so the same struct doubles as the TOML shape). `load_config` reads the file from `$WL_CROSSHAIR_CONFIG`, else `$XDG_CONFIG_HOME/wl-crosshair/config.toml`, else `~/.config/wl-crosshair/config.toml` (missing file is not an error — a malformed one is, and panics with the path). `resolve_settings` merges CLI over file over defaults, and is where the image-path fallback chain (`WL_CROSSHAIR_IMAGE_PATH` env var → compile-time `option_env!` default → `cursors/inverse-v.png` on disk) and the required `screen_width`/`screen_height` checks live. Missing required settings or bad parses are a hard `panic!` — this CLI intentionally fails fast/loud rather than validating gracefully. See `config.example.toml` for the file format.

2. **Wayland registry binding** — `Dispatch<wl_registry::WlRegistry, ()>` binds four globals as they're advertised: `zwlr_layer_shell_v1` (the overlay mechanism), `wl_compositor` (to create the surface), `wl_shm` (shared-memory buffer for the pixel data — this is where `State::draw` is invoked into a `tempfile`), and `xdg_wm_base`. All other Wayland object types get a no-op logging `Dispatch` impl via the `impl_dispatch_log!` macro at the bottom of the file — extend that macro list if you bind a new protocol object and just need visibility into its events.

3. **`State::draw`** — loads the image via the `image` crate, optionally resizes it (`--size`, Lanczos3), and writes it into the shm buffer as premultiplied-looking ARGB8888 (converts each pixel's RGBA to a big-endian-packed `u32` then writes little-endian bytes).

4. **`State::init_layer_surface`** — only runs once both `layer_shell` and `wm_base` globals have been seen after the first `blocking_dispatch`. Computes the surface's pixel position from `screen_width`/`screen_height`, the image's own size, and the `--offset-x/y` args, then anchors the layer surface top-left and uses `set_margin` to position it (there's no top-level positioning API in `wlr-layer-shell`, so margins from a top-left anchor are the positioning mechanism). Sets an empty input region so the overlay is click-through. Panics if the computed position would be negative (usually means wrong `--screen-width/height`).

5. **Main loop** — a single `event_queue.blocking_dispatch` loop; `state.running` flips to `false` on `zwlr_layer_surface_v1::Event::Closed`, which ends the program.

## Conventions

- No `unwrap`-avoidance discipline here by design — bad CLI input or an unexpected protocol state is expected to panic with a descriptive message rather than be handled gracefully.
- Keep new Wayland protocol object types wired through the `impl_dispatch_log!` macro unless they need real event handling, to keep `main.rs` from growing boilerplate `Dispatch` impls.
