# wl-crosshair

A tiny, click-through crosshair overlay for Wayland.

It puts an image of your choice in the middle of the screen, always on top and without catching any mouse clicks. It is useful for games that have no crosshair of their own, or for anything else that needs a fixed reference point.

![Default crosshair](https://github.com/lelgenio/wl-crosshair/assets/31388299/6e0aaa16-837b-40a8-9a13-ed808ea5db86)

> 🇩🇪 [Deutsche Kurzfassung weiter unten](#deutsch)

## Features

- **Centered automatically.** You don't need to set your screen resolution, and panels and docks are ignored.
- **Click-through.** The overlay never steals mouse or keyboard input.
- **Any image.** PNG, JPG and other formats work, and transparency is supported.
- **Offset and size.** Move the crosshair away from the center and scale the image to any size.
- **Config file and CLI flags.** Set it up once, then override single values when needed.
- **Toggle script.** Show or hide the crosshair with a keyboard shortcut.
- **Small.** About 400 lines of Rust in a single file, with no background daemon.

## Requirements

You need a Wayland compositor that supports **wlr-layer-shell**:

| Desktop | Status |
|---|---|
| KDE Plasma (Wayland) | ✅ tested |
| sway, Hyprland, river, other wlroots-based compositors | ✅ supported |
| GNOME | ❌ no layer-shell support |
| X11 | ❌ Wayland only |

## Installation

### From source (Cargo)

```sh
git clone https://github.com/fromaaage/wl-crosshair.git
cd wl-crosshair
cargo build --release
install -Dm755 target/release/wl-crosshair ~/.local/bin/wl-crosshair
install -Dm755 scripts/wl-crosshair-toggle ~/.local/bin/wl-crosshair-toggle
```

Make sure `~/.local/bin` is in your `PATH`.

### Nix

```sh
nix run github:fromaaage/wl-crosshair
```

## Quick start

```sh
mkdir -p ~/.config/wl-crosshair
cp dot.png ~/.config/wl-crosshair/
cat > ~/.config/wl-crosshair/config.toml <<'EOF'
image_path = "/home/YOU/.config/wl-crosshair/dot.png"
size = 24
EOF

wl-crosshair
```

Press `Ctrl+C` to stop it, or use the toggle script (see below).

## Configuration

The config file is read from the first of these paths that exists:

1. `$WL_CROSSHAIR_CONFIG`
2. `$XDG_CONFIG_HOME/wl-crosshair/config.toml`
3. `~/.config/wl-crosshair/config.toml`

| Key | CLI flag | Default | Description |
|---|---|---|---|
| `image_path` | `[IMAGE]` (positional) | see below | Path to the crosshair image |
| `size` | `--size <px>` | original size | Resize the image to `px` × `px` |
| `offset_x` | `--offset-x <px>` | `0` | Shift from the center: positive = right, negative = left |
| `offset_y` | `--offset-y <px>` | `0` | Shift from the center: positive = down, negative = up |

CLI flags always override the config file. If `image_path` is not set anywhere, `wl-crosshair` falls back to the `WL_CROSSHAIR_IMAGE_PATH` environment variable, then to `./cursors/inverse-v.png`.

Other flags:

- `-v`, `--verbose` prints Wayland debug output.
- `-V`, `--version` prints the version.
- `-h`, `--help` shows the help.

See [`config.example.toml`](./config.example.toml) for a commented example.

> **Upgrading from 0.1:** `screen_width` and `screen_height` are no longer needed. They are still accepted but ignored, and you can delete them from your config.

## Toggle with a keyboard shortcut

`scripts/wl-crosshair-toggle` starts the crosshair if it is hidden and stops it if it is shown. Bind it to a key in your desktop's shortcut settings.

**KDE Plasma:** go to *System Settings → Keyboard → Shortcuts → Add New → Command or Script*, enter `wl-crosshair-toggle` and assign a key.

**sway:**

```
bindsym $mod+x exec wl-crosshair-toggle
```

**Hyprland:**

```
bind = $mainMod, X, exec, wl-crosshair-toggle
```

## Custom crosshairs

Any image works. Here are some tips:

- Use a **PNG with transparency**. Semi-transparent edges are rendered correctly.
- Keep it small. 16–32 px usually looks best, and you can use `size` to scale it.
- Some examples are included in [`cursors/`](./cursors) and [`dot.png`](./dot.png).

![dot.png](https://raw.githubusercontent.com/fromaaage/wl-crosshair/refs/heads/main/dot.png)

## Troubleshooting

| Message | Fix |
|---|---|
| `compositor does not support wlr-layer-shell` | Your desktop has no overlay support. This is the case on GNOME. |
| `cannot connect to the Wayland compositor` | You are not in a Wayland session, or `WAYLAND_DISPLAY` is not set. |
| `invalid config file … unknown field` | There is a typo in a key in `config.toml`. The message shows the valid keys. |
| Crosshair is not where you expect | Adjust `offset_x` / `offset_y`. Note that the offset is in screen pixels *after* display scaling. |

## About this fork

This is a fork of [lelgenio/wl-crosshair](https://github.com/lelgenio/wl-crosshair), which is itself a heavily stripped-down take on [crossover](https://github.com/lacymorrow/crossover).

Changes in this fork:

- Config file support (TOML)
- Automatic centering, so no screen size is needed
- Offset and size options
- Readable error messages, plus a `--verbose` mode
- Toggle script

## License

[MIT](./LICENSE)

---

<a name="deutsch"></a>
## 🇩🇪 Deutsch (Kurzfassung)

**wl-crosshair** legt ein beliebiges Bild als Fadenkreuz in die Bildschirmmitte. Es liegt immer im Vordergrund, und Klicks gehen durch. Praktisch ist das für Spiele ohne eigenes Fadenkreuz.

- **Läuft auf:** KDE Plasma (Wayland), sway, Hyprland und anderen Compositors mit *wlr-layer-shell*. GNOME und X11 werden nicht unterstützt.
- **Installieren:** siehe [Installation](#installation). Das Programm wird mit `cargo build --release` gebaut und nach `~/.local/bin` kopiert.
- **Einrichten:** In `~/.config/wl-crosshair/config.toml` tragst du `image_path` ein, optional auch `size`, `offset_x` und `offset_y`. Die Bildschirmauflösung brauchst du nicht anzugeben.
- **An- und ausschalten per Taste (KDE):** *Systemeinstellungen → Tastatur → Kurzbefehle → Neu hinzufügen → Befehl oder Skript*. Dort `wl-crosshair-toggle` eintragen und eine Taste zuweisen.
- **Bei Problemen:** Starte das Programm mit `wl-crosshair --verbose`. Die Fehlermeldungen sagen dir, was nicht passt.
