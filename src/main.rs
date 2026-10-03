use std::{
    io::Write,
    os::unix::prelude::AsRawFd,
    path::PathBuf,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use image::{GenericImageView, Pixel};
use serde::Deserialize;
use wayland_client::{
    protocol::{wl_buffer, wl_compositor, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, Layer},
    zwlr_layer_surface_v1,
};

type Result<T> = std::result::Result<T, String>;

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Prints only with `--verbose`.
macro_rules! debug {
    ($($arg:tt)*) => {
        if VERBOSE.load(Ordering::Relaxed) {
            eprintln!($($arg)*);
        }
    };
}

const HELP: &str = "\
wl-crosshair – a click-through crosshair overlay for Wayland (wlr-layer-shell)

Usage: wl-crosshair [OPTIONS] [IMAGE]

Settings are read from the config file first; flags override single values.

Options:
  --offset-x <px>   Horizontal offset from the screen center (+ right, - left)
  --offset-y <px>   Vertical offset from the screen center (+ down, - up)
  --size <px>       Resize the image to <px> x <px>
  -v, --verbose     Print Wayland debug output
  -V, --version     Print version
  -h, --help        Show this help

Config file (first one found is used):
  $WL_CROSSHAIR_CONFIG
  $XDG_CONFIG_HOME/wl-crosshair/config.toml
  ~/.config/wl-crosshair/config.toml

Example:
  wl-crosshair --size 24 --offset-y 100 ~/.config/wl-crosshair/dot.png";

/// Shape of the config file; also used to collect CLI flags.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    image_path: Option<String>,
    offset_x: Option<i32>,
    offset_y: Option<i32>,
    size: Option<u32>,
    /// No longer needed (the compositor centers the overlay); accepted for old configs.
    screen_width: Option<u32>,
    screen_height: Option<u32>,
}

struct Settings {
    image_path: String,
    offset_x: i32,
    offset_y: i32,
    size: Option<u32>,
}

/// Pixel data for the shm buffer (premultiplied ARGB8888, little endian).
struct Frame {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

struct State {
    running: bool,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("wl-crosshair: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = parse_cli_args()?;
    let file = load_config()?;
    let settings = resolve_settings(cli, file)?;
    let frame = render(&settings)?;

    let conn = Connection::connect_to_env().map_err(|e| {
        format!("cannot connect to the Wayland compositor ({e}). Is this a Wayland session?")
    })?;
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    conn.display().get_registry(&qh, ());

    let mut state = State {
        running: true,
        compositor: None,
        shm: None,
        layer_shell: None,
        surface: None,
        buffer: None,
    };
    queue
        .roundtrip(&mut state)
        .map_err(|e| format!("Wayland error: {e}"))?;

    let compositor = state
        .compositor
        .clone()
        .ok_or("compositor does not offer wl_compositor")?;
    let shm = state
        .shm
        .clone()
        .ok_or("compositor does not offer wl_shm")?;
    let layer_shell = state.layer_shell.clone().ok_or(
        "compositor does not support wlr-layer-shell, which is required for overlays \
         (works on KDE Plasma, sway, Hyprland, … – not on GNOME)",
    )?;

    let mut file = tempfile::tempfile().map_err(|e| format!("cannot create buffer file: {e}"))?;
    file.write_all(&frame.pixels)
        .map_err(|e| format!("cannot write buffer file: {e}"))?;
    let pool = shm.create_pool(file.as_raw_fd(), frame.pixels.len() as i32, &qh, ());
    let buffer = pool.create_buffer(
        0,
        frame.width as i32,
        frame.height as i32,
        (frame.width * 4) as i32,
        wl_shm::Format::Argb8888,
        &qh,
        (),
    );

    let surface = compositor.create_surface(&qh, ());
    let layer =
        layer_shell.get_layer_surface(&surface, None, Layer::Overlay, "crosshair".into(), &qh, ());
    // No anchor: the compositor centers the surface on the output.
    // Exclusive zone -1: ignore panels, so it is the center of the whole screen.
    layer.set_size(frame.width, frame.height);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);

    // Empty input region = click-through.
    let region = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&region));
    region.destroy();
    surface.commit();

    state.surface = Some(surface);
    state.buffer = Some(buffer);

    while state.running {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| format!("Wayland error: {e}"))?;
    }
    Ok(())
}

fn parse_num<T: FromStr>(flag: &str, value: Option<String>) -> Result<T> {
    let value = value.ok_or_else(|| format!("missing value for {flag}"))?;
    value
        .parse()
        .map_err(|_| format!("invalid number for {flag}: '{value}'"))
}

fn parse_cli_args() -> Result<Config> {
    let mut args = std::env::args().skip(1);
    let mut cli = Config::default();

    while let Some(arg) = args.next() {
        // Accept both "--flag value" and "--flag=value"
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = || inline.clone().or_else(|| args.next());

        match flag.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("wl-crosshair {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-v" | "--verbose" => VERBOSE.store(true, Ordering::Relaxed),
            "--offset-x" => cli.offset_x = Some(parse_num(&flag, value())?),
            "--offset-y" => cli.offset_y = Some(parse_num(&flag, value())?),
            "--size" => cli.size = Some(parse_num(&flag, value())?),
            "--screen-width" => cli.screen_width = Some(parse_num(&flag, value())?),
            "--screen-height" => cli.screen_height = Some(parse_num(&flag, value())?),
            _ if flag.starts_with('-') => {
                return Err(format!("unknown option: {arg} (see --help)"));
            }
            _ if cli.image_path.is_none() => cli.image_path = Some(arg),
            _ => return Err(format!("unexpected extra argument: {arg}")),
        }
    }
    Ok(cli)
}

fn config_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("WL_CROSSHAIR_CONFIG") {
        return Some(PathBuf::from(path));
    }
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config_home).join("wl-crosshair/config.toml"));
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config/wl-crosshair/config.toml"))
}

fn load_config() -> Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            debug!("using config file {}", path.display());
            toml::from_str(&contents)
                .map_err(|e| format!("invalid config file {}:\n{e}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("cannot read config file {}: {e}", path.display())),
    }
}

fn resolve_settings(cli: Config, file: Config) -> Result<Settings> {
    if cli
        .screen_width
        .or(cli.screen_height)
        .or(file.screen_width)
        .or(file.screen_height)
        .is_some()
    {
        eprintln!(
            "wl-crosshair: note: screen_width/screen_height are no longer needed and are ignored \
             – the crosshair is centered automatically"
        );
    }

    let image_path = cli
        .image_path
        .or(file.image_path)
        .or_else(|| std::env::var("WL_CROSSHAIR_IMAGE_PATH").ok())
        .or_else(|| {
            [
                option_env!("WL_CROSSHAIR_IMAGE_PATH").map(String::from),
                Some("cursors/inverse-v.png".to_string()),
            ]
            .into_iter()
            .flatten()
            .find(|p| std::path::Path::new(p).is_file())
        })
        .ok_or(
            "no crosshair image found: pass it as an argument, set image_path in the config file, \
             or set WL_CROSSHAIR_IMAGE_PATH",
        )?;

    let size = cli.size.or(file.size);
    if size == Some(0) {
        return Err("size must be greater than 0".into());
    }

    Ok(Settings {
        image_path,
        offset_x: cli.offset_x.or(file.offset_x).unwrap_or(0),
        offset_y: cli.offset_y.or(file.offset_y).unwrap_or(0),
        size,
    })
}

/// Loads the image and places it in a transparent frame so that the frame's
/// center is the screen center and the image sits `offset` pixels away from it.
fn render(settings: &Settings) -> Result<Frame> {
    let mut img = image::open(&settings.image_path)
        .map_err(|e| format!("cannot open image {}: {e}", settings.image_path))?;
    if let Some(size) = settings.size {
        img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    }

    let (pad_x, pad_y) = (
        settings.offset_x.unsigned_abs(),
        settings.offset_y.unsigned_abs(),
    );
    let width = img.width() + 2 * pad_x;
    let height = img.height() + 2 * pad_y;
    let origin_x = (pad_x as i32 + settings.offset_x) as u32;
    let origin_y = (pad_y as i32 + settings.offset_y) as u32;
    debug!(
        "image {}x{}, frame {width}x{height}, image at {origin_x},{origin_y}",
        img.width(),
        img.height()
    );

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for (x, y, px) in img.pixels() {
        let [r, g, b, a] = px.to_rgba().0;
        let alpha = a as f32 / u8::MAX as f32;
        let premultiply = |c: u8| (c as f32 * alpha).round() as u8;
        let color = u32::from_be_bytes([a, premultiply(r), premultiply(g), premultiply(b)]);

        let i = (((origin_y + y) * width + origin_x + x) * 4) as usize;
        pixels[i..i + 4].copy_from_slice(&color.to_le_bytes());
    }

    Ok(Frame {
        pixels,
        width,
        height,
    })
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        debug!("global {interface} v{version}");

        if interface == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name {
            let v = version.min(zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().version);
            state.layer_shell = Some(registry.bind(name, v, qh, ()));
        } else if interface == wl_compositor::WlCompositor::interface().name {
            let v = version.min(wl_compositor::WlCompositor::interface().version);
            state.compositor = Some(registry.bind(name, v, qh, ()));
        } else if interface == wl_shm::WlShm::interface().name {
            state.shm = Some(registry.bind(name, 1, qh, ()));
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        layer: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        debug!("layer surface event {event:?}");
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                layer.ack_configure(serial);
                if let (Some(surface), Some(buffer)) = (&state.surface, &state.buffer) {
                    surface.attach(Some(buffer), 0, 0);
                    surface.commit();
                }
            }
            zwlr_layer_surface_v1::Event::Closed => state.running = false,
            _ => {}
        }
    }
}

/// Objects whose events we don't need – only logged with --verbose.
macro_rules! impl_dispatch_log {
    ($($t:path),* $(,)?) => {$(
        impl Dispatch<$t, ()> for State {
            fn event(
                _: &mut Self,
                _: &$t,
                event: <$t as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                debug!("{} event {event:?}", stringify!($t));
            }
        }
    )*};
}

impl_dispatch_log!(
    wl_buffer::WlBuffer,
    wl_compositor::WlCompositor,
    wl_region::WlRegion,
    wl_shm_pool::WlShmPool,
    wl_shm::WlShm,
    wl_surface::WlSurface,
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
);

#[cfg(test)]
mod tests {
    use super::*;

    /// 2x2 opaque white image, returns which frame pixels are set.
    fn rendered(offset_x: i32, offset_y: i32) -> (Frame, Vec<(u32, u32)>) {
        let path = std::env::temp_dir().join(format!("wl-crosshair-test-{offset_x}-{offset_y}.png"));
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]))
            .save(&path)
            .unwrap();
        let frame = render(&Settings {
            image_path: path.to_string_lossy().into(),
            offset_x,
            offset_y,
            size: None,
        })
        .unwrap();
        let set = (0..frame.height)
            .flat_map(|y| (0..frame.width).map(move |x| (x, y)))
            .filter(|(x, y)| frame.pixels[((y * frame.width + x) * 4 + 3) as usize] != 0)
            .collect();
        (frame, set)
    }

    #[test]
    fn no_offset_is_just_the_image() {
        let (frame, set) = rendered(0, 0);
        assert_eq!((frame.width, frame.height), (2, 2));
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn offset_moves_image_away_from_frame_center() {
        // 2x2 image, 3 px down and 1 px left → frame 4x8, image at x 0..2, y 6..8
        let (frame, set) = rendered(-1, 3);
        assert_eq!((frame.width, frame.height), (4, 8));
        assert_eq!(set, vec![(0, 6), (1, 6), (0, 7), (1, 7)]);
        // image center (1, 7) minus frame center (2, 4) = offset (-1, 3)
    }

    #[test]
    fn premultiplies_alpha() {
        let path = std::env::temp_dir().join("wl-crosshair-test-alpha.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 100, 0, 128]))
            .save(&path)
            .unwrap();
        let frame = render(&Settings {
            image_path: path.to_string_lossy().into(),
            offset_x: 0,
            offset_y: 0,
            size: None,
        })
        .unwrap();
        // little endian ARGB = [B, G, R, A]
        assert_eq!(frame.pixels, vec![0, 50, 100, 128]);
    }
}
