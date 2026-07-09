# wl-crosshair (€dit)
A crosshair overlay for wlroots compositors (like sway).

A extremely stripped down version of [crossover](https://github.com/lacymorrow/crossover).

## Install

```sh
cargo build --release
mkdir -p ~/.local/bin
cp target/release/wl-crosshair ~/.local/bin/wl-crosshair
```

Make sure `~/.local/bin` is in your `PATH` (`echo $PATH`).

## Quick start (config file, recommended)

Create `~/.config/wl-crosshair/config.toml`:

```toml
image_path = "/home/you/.config/wl-crosshair/dot.png"
screen_width = 1920
screen_height = 1080
```

Then just run, with no arguments:

```sh
wl-crosshair
```

Config file lookup order (first one found wins):
- `$WL_CROSSHAIR_CONFIG` (explicit path)
- `$XDG_CONFIG_HOME/wl-crosshair/config.toml`
- `~/.config/wl-crosshair/config.toml`

See [`config.example.toml`](./config.example.toml) for every available field
(`offset_x`/`offset_y`, `size`, ...) with explanations.

## CLI flags (optional, override the config file)

Flags are only needed for a one-off run without touching the config file, or
to override a single value for that run — any flag passed wins over the
matching config field:

- `--screen-width <px>`: width of the target screen
- `--screen-height <px>`: height of the target screen
- `--size <px>`: resize the image to size×size before rendering (optional)
- `--offset-x <px>`: horizontal offset from center, positive = right, negative = left (optional)
- `--offset-y <px>`: vertical offset from center, positive = down, negative = up (optional)
- `<image-path>`: path to the crosshair image (optional)

```sh
# fully via flags, no config file needed
wl-crosshair --screen-width 1920 --screen-height 1080 --size 24 ./dot.png
```

### Preview (default cursor):
![image](https://github.com/lelgenio/wl-crosshair/assets/31388299/6e0aaa16-837b-40a8-9a13-ed808ea5db86)

### Alternative crosshair dot.png:
![image](https://raw.githubusercontent.com/fromaaage/wl-crosshair/refs/heads/main/dot.png)

## TODO
- [x] Make the crosshair Click-through https://github.com/lelgenio/wl-crosshair/pull/1
- [x] Option to control size of crosshair
- [x] Option to offset crosshair
- [x] Configuration file
- [x] Support for loading custom crosshair images
