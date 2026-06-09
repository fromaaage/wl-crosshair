# wl-crosshair (€dit)
A crosshair overlay for wlroots compositors (like sway).

A extremely stripped down version of [crossover](https://github.com/lacymorrow/crossover).

```sh
wl-crosshair ./my-crosshair.png
```

### Preview (default cursor):
![image](https://github.com/lelgenio/wl-crosshair/assets/31388299/6e0aaa16-837b-40a8-9a13-ed808ea5db86)

### Alternative crosshair dot.png:
![image](https://raw.githubusercontent.com/fromaaage/wl-crosshair/refs/heads/main/dot.png)

## TODO
- [x] Make the crosshair Click-through https://github.com/lelgenio/wl-crosshair/pull/1
- [x] Option to control size of crosshair
- [x] Option to offset crosshair
- [ ] Configuration file
- [x] Support for loading custom crosshair images


## €dit / Addition - Arguments

- `--screen-width <px>`: width of the target screen
- `--screen-height <px>`: height of the target screen
- `--size <px>`: resize the image to size×size before rendering (optional)
- `--offset-x <px>`: horizontal offset from center, positive = right, negative = left (optional)
- `--offset-y <px>`: vertical offset from center, positive = down, negative = up (optional)
- `<image-path>`: path to the crosshair image

## Examples

```bash
# 1920x1080, centered
wl-crosshair --screen-width 1920 --screen-height 1080 ./dot.png

# 1920x1080, centered, resized to 24x24
wl-crosshair --screen-width 1920 --screen-height 1080 --size 24 ./dot.png

# 2560x1440, centered, resized to 12x12
wl-crosshair --screen-width 2560 --screen-height 1440 --size 12 ./dot.png

# 2560x1440, centered, resized to 24x24, with fine offset
wl-crosshair --screen-width 2560 --screen-height 1440 --size 24 --offset-x 2 --offset-y -1 ./dot.png
```
