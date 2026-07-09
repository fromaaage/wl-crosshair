use std::{fs::File, io::Write, os::unix::prelude::AsRawFd, path::PathBuf};

use image::{GenericImageView, Pixel};
use serde::Deserialize;
use wayland_client::{
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_region::WlRegion, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use wayland_protocols::xdg::shell::client::xdg_wm_base;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, Layer},
    zwlr_layer_surface_v1::{self, Anchor},
};

struct State {
    running: bool,

    cursor_width: u32,
    cursor_height: u32,
    image_path: String,
    offset_x: i32,
    offset_y: i32,
    forced_size: Option<u32>,
        screen_width: u32,
        screen_height: u32,

        compositor: Option<wl_compositor::WlCompositor>,
        base_surface: Option<wl_surface::WlSurface>,
        layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
        buffer: Option<wl_buffer::WlBuffer>,
        wm_base: Option<xdg_wm_base::XdgWmBase>,
}

fn print_help_and_exit() -> ! {
    eprintln!(
        "Usage: wl-crosshair [OPTIONS] [image-path]

        Options are read from a config file first, then overridden by any
        of these flags:
        --screen-width <px>   Width of the target screen in pixels
        --screen-height <px>  Height of the target screen in pixels
        --offset-x <px>       Horizontal offset from center, positive = right, negative = left
        --offset-y <px>       Vertical offset from center, positive = down, negative = up
        --size <px>           Resize image to a square size x size
        -h, --help            Show this help

        Config file location (first one found is used):
        - $WL_CROSSHAIR_CONFIG (explicit path override)
        - $XDG_CONFIG_HOME/wl-crosshair/config.toml
        - ~/.config/wl-crosshair/config.toml
        See config.example.toml for the file format.

        Example:
        ./wl-crosshair --screen-width 1920 --screen-height 1080 --size 24 ./dot.png"
    );
    std::process::exit(0);
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    image_path: Option<String>,
    offset_x: Option<i32>,
    offset_y: Option<i32>,
    size: Option<u32>,
    screen_width: Option<u32>,
    screen_height: Option<u32>,
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

fn load_config() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents)
        .unwrap_or_else(|e| panic!("Invalid config file '{}': {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => panic!("Could not read config file '{}': {e}", path.display()),
    }
}

fn parse_cli_args() -> Config {
    let mut args = std::env::args().skip(1).peekable();
    let mut cli = Config::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => print_help_and_exit(),
            "--offset-x" => {
                cli.offset_x = Some(
                    args.next()
                    .expect("Missing value for --offset-x")
                    .parse::<i32>()
                    .expect("Invalid integer for --offset-x"),
                );
            }
            "--offset-y" => {
                cli.offset_y = Some(
                    args.next()
                    .expect("Missing value for --offset-y")
                    .parse::<i32>()
                    .expect("Invalid integer for --offset-y"),
                );
            }
            "--size" => {
                cli.size = Some(
                    args.next()
                    .expect("Missing value for --size")
                    .parse::<u32>()
                    .expect("Invalid integer for --size"),
                );
            }
            "--screen-width" => {
                cli.screen_width = Some(
                    args.next()
                    .expect("Missing value for --screen-width")
                    .parse::<u32>()
                    .expect("Invalid integer for --screen-width"),
                );
            }
            "--screen-height" => {
                cli.screen_height = Some(
                    args.next()
                    .expect("Missing value for --screen-height")
                    .parse::<u32>()
                    .expect("Invalid integer for --screen-height"),
                );
            }
            _ if arg.starts_with("--offset-x=") => {
                cli.offset_x = Some(
                    arg["--offset-x=".len()..]
                    .parse::<i32>()
                    .expect("Invalid integer for --offset-x"),
                );
            }
            _ if arg.starts_with("--offset-y=") => {
                cli.offset_y = Some(
                    arg["--offset-y=".len()..]
                    .parse::<i32>()
                    .expect("Invalid integer for --offset-y"),
                );
            }
            _ if arg.starts_with("--size=") => {
                cli.size = Some(
                    arg["--size=".len()..]
                    .parse::<u32>()
                    .expect("Invalid integer for --size"),
                );
            }
            _ if arg.starts_with("--screen-width=") => {
                cli.screen_width = Some(
                    arg["--screen-width=".len()..]
                    .parse::<u32>()
                    .expect("Invalid integer for --screen-width"),
                );
            }
            _ if arg.starts_with("--screen-height=") => {
                cli.screen_height = Some(
                    arg["--screen-height=".len()..]
                    .parse::<u32>()
                    .expect("Invalid integer for --screen-height"),
                );
            }
            _ if arg.starts_with('-') => {
                panic!("Unknown option: {arg}");
            }
            _ => {
                if cli.image_path.is_none() {
                    cli.image_path = Some(arg);
                } else {
                    panic!("Unexpected extra positional argument: {arg}");
                }
            }
        }
    }

    cli
}

fn resolve_settings(cli: Config, file: Config) -> (String, i32, i32, Option<u32>, u32, u32) {
    let image_path = cli
    .image_path
    .or(file.image_path)
    .or_else(|| std::env::var("WL_CROSSHAIR_IMAGE_PATH").ok())
    .or_else(|| {
        [
            std::option_env!("WL_CROSSHAIR_IMAGE_PATH").map(String::from),
             Some("cursors/inverse-v.png".to_string()),
        ]
        .into_iter()
        .flatten()
        .find(|p| std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false))
    })
    .expect(
        "Could not find a crosshair image, pass it as a cli argument, set image_path in the config file, or set WL_CROSSHAIR_IMAGE_PATH",
    );

    let offset_x = cli.offset_x.or(file.offset_x).unwrap_or(0);
    let offset_y = cli.offset_y.or(file.offset_y).unwrap_or(0);
    let forced_size = cli.size.or(file.size);

    let screen_width = cli.screen_width.or(file.screen_width).expect(
        "Missing screen width: pass --screen-width, or set screen_width in the config file",
    );
    let screen_height = cli.screen_height.or(file.screen_height).expect(
        "Missing screen height: pass --screen-height, or set screen_height in the config file",
    );

    (image_path, offset_x, offset_y, forced_size, screen_width, screen_height)
}

fn main() {
    let cli = parse_cli_args();
    let file_config = load_config();
    let (image_path, offset_x, offset_y, forced_size, screen_width, screen_height) =
    resolve_settings(cli, file_config);

    let conn = Connection::connect_to_env().unwrap();

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = State {
        running: true,
        cursor_width: 10,
        cursor_height: 10,
        image_path,
        offset_x,
        offset_y,
        forced_size,
            screen_width,
            screen_height,
            compositor: None,
            base_surface: None,
            layer_shell: None,
            layer_surface: None,
            buffer: None,
            wm_base: None,
    };

    event_queue.blocking_dispatch(&mut state).unwrap();

    if state.layer_shell.is_some() && state.wm_base.is_some() {
        state.init_layer_surface(&qhandle);
    }

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
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
        eprintln!("WlRegistry event {event:#?}");
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == zwlr_layer_shell_v1::ZwlrLayerShellV1::interface().name {
                let wl_layer = registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                    name,
                    version,
                    qh,
                    (),
                );
                state.layer_shell = Some(wl_layer);
            } else if interface == wl_compositor::WlCompositor::interface().name {
                let compositor =
                registry.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ());
                let surface = compositor.create_surface(qh, ());
                state.base_surface = Some(surface);
                state.compositor = Some(compositor);
            } else if interface == wl_shm::WlShm::interface().name {
                let shm = registry.bind::<wl_shm::WlShm, _, _>(name, version, qh, ());

                let mut file = tempfile::tempfile().unwrap();
                state.draw(&mut file);

                let (init_w, init_h) = (state.cursor_width, state.cursor_height);

                let pool = shm.create_pool(file.as_raw_fd(), (init_w * init_h * 4) as i32, qh, ());
                let buffer = pool.create_buffer(
                    0,
                    init_w as i32,
                    init_h as i32,
                    (init_w * 4) as i32,
                                                wl_shm::Format::Argb8888,
                                                qh,
                                                (),
                );
                state.buffer = Some(buffer);
            } else if interface == xdg_wm_base::XdgWmBase::interface().name {
                let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                state.wm_base = Some(wm_base);
            }
        }
    }
}

impl Dispatch<WlRegion, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlRegion,
        _: <WlRegion as Proxy>::Event,
        _: &(),
             _: &Connection,
             _: &QueueHandle<Self>,
    ) {
    }
}

impl State {
    fn init_layer_surface(&mut self, qh: &QueueHandle<State>) {
        let layer = self.layer_shell.as_ref().unwrap().get_layer_surface(
            self.base_surface.as_ref().unwrap(),
                                                                         None,
                                                                         Layer::Overlay,
                                                                         "crosshair".to_string(),
                                                                         qh,
                                                                         (),
        );

        layer.set_anchor(Anchor::Top | Anchor::Left);
        layer.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        layer.set_size(self.cursor_width, self.cursor_height);
        layer.set_exclusive_zone(0);

        let pos_x =
        (self.screen_width as i32 / 2) - (self.cursor_width as i32 / 2) + self.offset_x;
        let pos_y =
        (self.screen_height as i32 / 2) - (self.cursor_height as i32 / 2) + self.offset_y;

        if pos_x < 0 || pos_y < 0 {
            panic!(
                "Calculated negative position: x={}, y={}. Check screen size and offsets.",
                pos_x, pos_y
            );
        }

        layer.set_margin(pos_y, 0, 0, pos_x);

        if let Some(compositor) = &self.compositor {
            let region = compositor.create_region(qh, ());
            self.base_surface
            .as_ref()
            .unwrap()
            .set_input_region(Some(&region));
        }

        self.base_surface.as_ref().unwrap().commit();
        self.layer_surface = Some(layer);
    }

    fn draw(&mut self, tmp: &mut File) {
        let mut buf = std::io::BufWriter::new(tmp);

        let mut i = image::open(&self.image_path)
        .unwrap_or_else(|e| panic!("Could not open image '{}': {e}", self.image_path));

        if let Some(size) = self.forced_size {
            i = i.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        }

        self.cursor_width = i.width();
        self.cursor_height = i.height();

        for y in 0..self.cursor_height {
            for x in 0..self.cursor_width {
                let px = i.get_pixel(x, y).to_rgba();

                let [r, g, b, a] = px.channels().try_into().unwrap();
                let color = u32::from_be_bytes([a, r, g, b]);

                buf.write_all(&color.to_le_bytes()).unwrap();
            }
        }
        buf.flush().unwrap();
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
             _conn: &Connection,
             _qhandle: &QueueHandle<Self>,
    ) {
        eprintln!("ZwlrLayerSurfaceV1 event {event:#?}");
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                surface.ack_configure(serial);
                if let (Some(surface), Some(buffer)) = (&state.base_surface, &state.buffer) {
                    surface.attach(Some(buffer), 0, 0);
                    surface.commit();
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => {}
        }
    }
}

macro_rules! impl_dispatch_log {
    ($DispatchStruct: path) => {
        impl Dispatch<$DispatchStruct, ()> for State {
            fn event(
                _: &mut Self,
                _: &$DispatchStruct,
                event: <$DispatchStruct as Proxy>::Event,
                _: &(),
                     _: &Connection,
                     _: &QueueHandle<Self>,
            ) {
                eprintln!("{} event {:#?}", stringify!($DispatchStruct), event);
            }
        }
    };
}

impl_dispatch_log!(wl_buffer::WlBuffer);
impl_dispatch_log!(wl_compositor::WlCompositor);
impl_dispatch_log!(wl_keyboard::WlKeyboard);
impl_dispatch_log!(wl_seat::WlSeat);
impl_dispatch_log!(wl_shm_pool::WlShmPool);
impl_dispatch_log!(wl_shm::WlShm);
impl_dispatch_log!(wl_surface::WlSurface);
impl_dispatch_log!(xdg_wm_base::XdgWmBase);
impl_dispatch_log!(zwlr_layer_shell_v1::ZwlrLayerShellV1);
