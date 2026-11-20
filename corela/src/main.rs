mod draw;
mod keys;

use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use bosonoga::Program;
use bosonoga::ast::ExitStatus;
use bosonoga::gfx::{Input, Scene, Window};
use bosonoga::settings::{Asset, Settings};
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use macroquad::input::{is_quit_requested, prevent_quit};
use macroquad::miniquad::{EventHandler, KeyCode, KeyMods};
use macroquad::window::{Conf, next_frame};

const SETTINGS_FILE: &str = "regina.ini";
const DEFAULT_PROGRAM: &str = "gfx/gfx_corela_bpmn.bosonoga";
const INTERPRETER_STACK: usize = 1 << 28;
const FRAME_TIME: Duration = Duration::from_micros(1_000_000 / 60);

enum Event {
    Info(String),
    Frame(Scene),
    Done(Result<ExitStatus, String>),
}

struct Channel {
    events: Sender<Event>,
    inputs: Receiver<Option<Input>>,
}

impl Window for Channel {
    fn info(&mut self, text: &str) {
        let _ = self.events.send(Event::Info(text.to_owned()));
    }

    fn show(&mut self, scene: &Scene) -> Option<Input> {
        self.events.send(Event::Frame(scene.clone())).ok()?;
        self.inputs.recv().ok().flatten()
    }
}

#[derive(Default)]
struct Keys {
    pressed: Vec<u32>,
}

impl EventHandler for Keys {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn key_down_event(&mut self, keycode: KeyCode, _: KeyMods, repeat: bool) {
        if let (false, Some(code)) = (repeat, keys::code(keycode)) {
            self.pressed.push(code);
        }
    }
}

fn exit_code(status: ExitStatus) -> u8 {
    match status {
        ExitStatus::Success => 0,
        ExitStatus::Failure => 1,
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn read_assets(
    path: &str,
    group: &str,
    assets: &[(String, Asset)],
) -> Result<Vec<(String, String, Vec<u8>)>, String> {
    assets
        .iter()
        .map(|(key, asset)| {
            let shown = asset.path.display().to_string();
            std::fs::read(&asset.path)
                .map(|bytes| (key.clone(), shown.clone(), bytes))
                .map_err(|error| format!("{path}: {group}_{key}: {shown}: {error}"))
        })
        .collect()
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let path = arguments
        .next()
        .unwrap_or_else(|| DEFAULT_PROGRAM.to_owned());
    if arguments.next().is_some() {
        return fail("usage: corela_bpmn [FILE]");
    }
    let settings = match Settings::read(Path::new(SETTINGS_FILE)) {
        Ok(settings) => settings,
        Err(message) => return fail(&message),
    };
    if settings.version != bosonoga::VERSION {
        return fail(&format!(
            "corela_bpmn {} needs bosonoga {}, but this is bosonoga {}",
            settings.version,
            settings.version,
            bosonoga::VERSION
        ));
    }
    let program = match bosonoga::load(Path::new(&path), &settings) {
        Ok(program) => program,
        Err(message) => return fail(&message),
    };
    if !program.is_gfx() {
        return fail(&format!("{path}: a gfx app needs gfx_main"));
    }
    let files = read_assets(&path, "font", program.fonts())
        .and_then(|fonts| Ok((fonts, read_assets(&path, "icon", program.icons())?)));
    let (fonts, icons) = match files {
        Ok(read) => read,
        Err(message) => return fail(&message),
    };
    let conf = Conf {
        window_title: path,
        window_width: i32::try_from(settings.gfx_width).unwrap_or(i32::MAX),
        window_height: i32::try_from(settings.gfx_height).unwrap_or(i32::MAX),
        ..Conf::default()
    };
    macroquad::Window::from_config(conf, run(program, fonts, icons));
    ExitCode::FAILURE
}

async fn run(
    program: Program,
    fonts: Vec<(String, String, Vec<u8>)>,
    icons: Vec<(String, String, Vec<u8>)>,
) {
    let assets = match draw::Assets::load(program.path(), &fonts, &icons) {
        Ok(assets) => assets,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };
    prevent_quit();
    let subscriber = register_input_subscriber();
    let (events, received) = channel();
    let (inputs, input_receiver) = channel();
    let done = events.clone();
    let started = thread::Builder::new()
        .stack_size(INTERPRETER_STACK)
        .spawn(move || {
            let mut window = Channel {
                events,
                inputs: input_receiver,
            };
            let result = bosonoga::gfx::run(&program, &mut window);
            let _ = done.send(Event::Done(result));
        });
    if let Err(error) = started {
        eprintln!("cannot start the interpreter: {error}");
        std::process::exit(1);
    }
    let mut keys = Keys::default();
    let mut next_frame_time = Instant::now();
    let mut last_frame_start: Option<Instant> = None;
    loop {
        match received.recv() {
            Ok(Event::Info(text)) => print!("{text}"),
            Ok(Event::Frame(scene)) => {
                draw::scene(&scene, &assets);
                next_frame().await;
                let now = Instant::now();
                if now < next_frame_time {
                    thread::sleep(next_frame_time - now);
                    next_frame_time += FRAME_TIME;
                } else {
                    next_frame_time = now + FRAME_TIME;
                }
                let start = Instant::now();
                let delta_time = last_frame_start
                    .map_or(FRAME_TIME, |last| start - last)
                    .as_secs_f32();
                last_frame_start = Some(start);
                repeat_all_miniquad_input(&mut keys, subscriber);
                let input = Input {
                    keys: std::mem::take(&mut keys.pressed),
                    delta_time,
                };
                let _ = inputs.send((!is_quit_requested()).then_some(input));
            }
            Ok(Event::Done(Ok(status))) => std::process::exit(exit_code(status).into()),
            Ok(Event::Done(Err(message))) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("the interpreter stopped without a result");
                std::process::exit(1);
            }
        }
    }
}
