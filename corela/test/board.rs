use std::path::Path;

use bosonoga::ast::ExitStatus;
use bosonoga::gfx::{Input, Scene, Shape, Window};
use bosonoga::settings::Settings;

const PROGRAM: &str = "gfx/gfx_corela_bpmn.bosonoga";

#[derive(Default)]
struct Script {
    keys: Vec<Vec<u32>>,
    closes: bool,
    frames: Vec<Scene>,
    info: String,
}

impl Window for Script {
    fn info(&mut self, text: &str) {
        self.info.push_str(text);
    }

    fn show(&mut self, scene: &Scene) -> Option<Input> {
        self.frames.push(scene.clone());
        let index = self.frames.len() - 1;
        if self.closes && index >= self.keys.len() {
            return None;
        }
        Some(Input {
            keys: self.keys.get(index).cloned().unwrap_or_default(),
            delta_time: 1.0 / 60.0,
        })
    }
}

#[cfg(test)]
fn run(keys: &[&[&str]], closes: bool) -> (Result<ExitStatus, String>, Script) {
    let settings = Settings::read(Path::new("regina.ini")).unwrap();
    let program = bosonoga::load(Path::new(PROGRAM), &settings).unwrap();
    let mut script = Script {
        keys: keys
            .iter()
            .map(|frame| {
                frame
                    .iter()
                    .map(|name| bosonoga::key::named(name).unwrap())
                    .collect()
            })
            .collect(),
        closes,
        ..Script::default()
    };
    let result = bosonoga::gfx::run(&program, &mut script);
    (result, script)
}

#[cfg(test)]
fn hexes(scene: &Scene, color: u32) -> Vec<(f32, f32)> {
    scene
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Hex { x, y, color: c, .. } if *c == color => Some((*x, *y)),
            Shape::Rectangle { .. }
            | Shape::Hex { .. }
            | Shape::Circle { .. }
            | Shape::Text { .. } => None,
        })
        .collect()
}

#[cfg(test)]
fn texts(scene: &Scene, color: u32) -> Vec<&str> {
    scene
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text { text, color: c, .. } if *c == color => Some(text.as_str()),
            Shape::Rectangle { .. }
            | Shape::Hex { .. }
            | Shape::Circle { .. }
            | Shape::Text { .. } => None,
        })
        .collect()
}

#[test]
fn closes_after_idle_close_ms() {
    let (result, script) = run(&[], false);
    assert_eq!(result, Ok(ExitStatus::Success));
    assert_eq!(script.frames.len(), 1 + 3200 * 60 / 1000);
    assert_eq!(script.info, "");
}

#[test]
fn draws_the_board_with_labels() {
    let (_, script) = run(&[], true);
    let first = &script.frames[0];
    assert_eq!(first.clear, 0x1E_1E2E);
    assert_eq!(hexes(first, 0x31_3244).len(), 60);
    assert_eq!(hexes(first, 0xFA_B387), [(576.0, 324.0)]);
    assert_eq!(texts(first, 0x1E_1E2E), ["E5"]);
    let faded = texts(first, 0x4A_4C60);
    assert_eq!(faded.len(), 60);
    assert_eq!(faded[..5], ["A5", "A4", "A3", "A2", "A1"]);
    assert!(faded.contains(&"E9") && !faded.contains(&"A6") && !faded.contains(&"I6"));
}

#[test]
fn keys_move_the_selection_and_switch_the_theme() {
    let north = "SAPP_KEYCODE_W";
    let (result, script) = run(
        &[
            &[north],
            &[north, north, north, north],
            &["SAPP_KEYCODE_D", "SAPP_KEYCODE_M"],
        ],
        true,
    );
    assert_eq!(
        result,
        Err(format!(
            "{PROGRAM}: the window was closed before the program returned"
        ))
    );
    assert_eq!(
        script.info,
        "core: selected_0_-1\n\
         core: selected_0_-2\n\
         core: selected_0_-3\n\
         core: selected_0_-4\n\
         core: 0_-5_is_out_of_bounds\n\
         core: selected_1_-4\n\
         core: light_theme\n"
    );
    let last = script.frames.last().unwrap();
    assert_eq!(last.clear, 0x30_3446);
    let [(x, y)] = hexes(last, 0xEF_9F76)[..] else {
        panic!("one selected tile");
    };
    assert!(
        (x - 624.0).abs() < 1e-3 && (y - 130.010_3).abs() < 1e-3,
        "{x} {y}"
    );
    assert_eq!(texts(last, 0x30_3446), ["F8"]);
}
