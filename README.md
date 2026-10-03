# wl-crosshair

Click-through crosshair overlay for Wayland. It works on KDE Plasma, sway, Hyprland and other compositors with `wlr-layer-shell`. It does not work on GNOME.

It is centered automatically, and you can **offset it** for games that aim off-center.

![Default crosshair](https://github.com/lelgenio/wl-crosshair/assets/31388299/6e0aaa16-837b-40a8-9a13-ed808ea5db86)

## Install

```sh
cargo build --release
install -Dm755 -t ~/.local/bin target/release/wl-crosshair scripts/wl-crosshair-toggle
```

Or with Nix: `nix run github:fromaaage/wl-crosshair`

## Config

`~/.config/wl-crosshair/config.toml`:

```toml
image_path = "/home/you/.config/wl-crosshair/dot.png"
size = 24        # optional, resize to 24x24
offset_x = 0     # optional, + right / - left
offset_y = 145   # optional, + down / - up
```

Every key also works as a flag, for example `--size 24`, `--offset-y 145` or a path to the image. Flags override the config file. Run `wl-crosshair --help` for all options.

## Toggle with a hotkey

Bind `wl-crosshair-toggle` to a key. Pressing it turns the crosshair on, pressing it again turns it off.

- **KDE:** System Settings → Keyboard → Shortcuts → Add New → Command or Script
- **sway:** `bindsym $mod+x exec wl-crosshair-toggle`
- **Hyprland:** `bind = $mainMod, X, exec, wl-crosshair-toggle`

## Notes

- Any image works. A small PNG with transparency looks best. See `dot.png` and `cursors/` for examples.
- If you are upgrading from 0.1, delete `screen_width` and `screen_height` from your config. They are no longer needed.

Fork of [lelgenio/wl-crosshair](https://github.com/lelgenio/wl-crosshair). MIT licensed.

---

**Deutsch:** Fadenkreuz-Overlay für Wayland (KDE, sway, Hyprland). Es zentriert sich automatisch, und mit `offset_x`/`offset_y` lässt es sich für Spiele verschieben. Die Einrichtung steht oben unter *Config*. Zum An- und Ausschalten legst du `wl-crosshair-toggle` auf eine Taste.
