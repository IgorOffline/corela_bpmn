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
        if self.events.send(Event::Frame(scene.clone())).is_err() {
            return None;
        }
        self.inputs.recv().unwrap_or(None)
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
        if repeat {
            return;
        }
        if let Some(code) = keys::code(keycode) {
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
    let mut files = Vec::new();
    for (key, asset) in assets {
        let shown = asset.path.display().to_string();
        match std::fs::read(&asset.path) {
            Ok(bytes) => files.push((key.clone(), shown, bytes)),
            Err(error) => return Err(format!("{path}: {group}_{key}: {shown}: {error}")),
        }
    }
    Ok(files)
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mut path = DEFAULT_PROGRAM.to_owned();
    if let Some(given) = arguments.next() {
        path = given;
    }
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
    let fonts = match read_assets(&path, "font", program.fonts()) {
        Ok(fonts) => fonts,
        Err(message) => return fail(&message),
    };
    let icons = match read_assets(&path, "icon", program.icons()) {
        Ok(icons) => icons,
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
    let mut pacer = Pacer {
        keys: Keys::default(),
        next_frame_time: Instant::now(),
        last_frame_start: None,
    };
    loop {
        match received.recv() {
            Ok(Event::Info(text)) => print!("{text}"),
            Ok(Event::Frame(scene)) => {
                draw::scene(&scene, &assets);
                next_frame().await;
                let input = pacer.wait(subscriber);
                if is_quit_requested() {
                    let _ = inputs.send(None);
                } else {
                    let _ = inputs.send(Some(input));
                }
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

struct Pacer {
    keys: Keys,
    next_frame_time: Instant,
    last_frame_start: Option<Instant>,
}

impl Pacer {
    fn wait(&mut self, subscriber: usize) -> Input {
        let now = Instant::now();
        if now < self.next_frame_time {
            thread::sleep(self.next_frame_time - now);
            self.next_frame_time += FRAME_TIME;
        } else {
            self.next_frame_time = now + FRAME_TIME;
        }
        let start = Instant::now();
        let elapsed = match self.last_frame_start {
            Some(last) => start - last,
            None => FRAME_TIME,
        };
        self.last_frame_start = Some(start);
        repeat_all_miniquad_input(&mut self.keys, subscriber);
        Input {
            keys: std::mem::take(&mut self.keys.pressed),
            delta_time: elapsed.as_secs_f32(),
        }
    }
}
