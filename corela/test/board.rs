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
            | Shape::Icon { .. }
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
            | Shape::Icon { .. }
            | Shape::Text { .. } => None,
        })
        .collect()
}

#[cfg(test)]
fn near(actual: (f32, f32), expected: (f32, f32)) -> bool {
    (actual.0 - expected.0).abs() < 1e-3 && (actual.1 - expected.1).abs() < 1e-3
}

#[test]
fn closes_after_idle_close_ms() {
    let (result, script) = run(&[], false);
    assert_eq!(result, Ok(ExitStatus::Success));
    assert_eq!(script.frames.len(), 1 + 3200 * 60 / 1000);
    assert_eq!(script.info, "");
}

#[test]
fn draws_the_board_and_the_ui_labels() {
    let (_, script) = run(&[], true);
    let first = &script.frames[0];
    assert_eq!(first.clear, 0x1E_1E2E);
    assert_eq!(
        texts(first, 0xCD_D6F4),
        ["Center", "Mode: None", "Action 1 / 24"]
    );
    assert_eq!(hexes(first, 0x31_3244).len(), 60);
    assert_eq!(hexes(first, 0xFA_B387), [(576.0, 324.0)]);
    assert_eq!(texts(first, 0x1E_1E2E), ["E5"]);
    let faded = texts(first, 0x4A_4C60);
    assert_eq!(faded.len(), 60);
    assert_eq!(faded[..5], ["A5", "A4", "A3", "A2", "A1"]);
    assert!(faded.contains(&"E9") && !faded.contains(&"A6") && !faded.contains(&"I6"));
}

#[test]
fn keys_edit_the_board_in_modes() {
    let north = "SAPP_KEYCODE_W";
    let arrow = "SAPP_KEYCODE_1";
    let cycle = "SAPP_KEYCODE_I";
    let (result, script) = run(
        &[
            &[north, cycle],
            &[north, north, north, north],
            &[north],
            &["SAPP_KEYCODE_0", "SAPP_KEYCODE_E"],
            &[arrow, arrow],
            &[cycle, "SAPP_KEYCODE_R"],
            &["SAPP_KEYCODE_M"],
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
        "core: action_1_of_24: input_mode_input\n\
         core: action_2_of_24: selected_0_-1\n\
         core: action_3_of_24: selected_0_-2\n\
         core: action_4_of_24: selected_0_-3\n\
         core: action_5_of_24: selected_0_-4\n\
         core: action_6_of_24: entered_north_region\n\
         core: action_7_of_24: placed_start_node_at_0_4\n\
         core: action_8_of_24: selected_1_3\n\
         core: action_9_of_24: placed_arrow_at_1_3\n\
         core: action_10_of_24: turned_arrow_at_1_3_to_1\n\
         core: action_10_of_24: input_mode_undo\n\
         core: action_11_of_24: undo_8_more_to_undo_1_to_redo\n\
         core: action_11_of_24: light_theme\n"
    );
    let last = script.frames.last().unwrap();
    assert_eq!(last.clear, 0x30_3446);
    assert_eq!(
        texts(last, 0xC6_D0F5),
        ["North", "Mode: Undo", "Action 11 / 24"]
    );
    let [selected] = hexes(last, 0xEF_9F76)[..] else {
        panic!("one selected tile");
    };
    assert!(near(selected, (624.0, 517.989_7)), "{selected:?}");
    let circles: Vec<(f32, f32, f32, u32)> = last
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Circle {
                x,
                y,
                radius,
                color,
            } => Some((*x, *y, *radius, *color)),
            Shape::Rectangle { .. }
            | Shape::Hex { .. }
            | Shape::Icon { .. }
            | Shape::Text { .. } => None,
        })
        .collect();
    assert_eq!(
        circles,
        [
            (576.0, 324.0 + 4.0 * 32.0 * 1.732_050_8, 15.75, 0xE7_8284),
            (576.0, 324.0 + 4.0 * 32.0 * 1.732_050_8, 14.25, 0xC6_D0F5),
        ]
    );
    let icons: Vec<(f32, f32, usize, f32, u32)> = last
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Icon {
                x,
                y,
                icon,
                angle,
                color,
            } => Some((*x, *y, *icon, *angle, *color)),
            Shape::Rectangle { .. }
            | Shape::Hex { .. }
            | Shape::Circle { .. }
            | Shape::Text { .. } => None,
        })
        .collect();
    let [(x, y, 0, angle, 0x30_3446)] = icons[..] else {
        panic!("one selected arrow, not {icons:?}");
    };
    assert!(near((x, y), selected) && angle == 0.0, "{icons:?}");
}

#[test]
fn the_last_action_ends_the_run() {
    let mut keys: Vec<&[&str]> = vec![&["SAPP_KEYCODE_I"]];
    for _ in 0..12 {
        keys.push(&["SAPP_KEYCODE_W"]);
        keys.push(&["SAPP_KEYCODE_S"]);
    }
    let (result, script) = run(&keys, true);
    assert_eq!(result, Ok(ExitStatus::Success));
    assert!(
        script.info.ends_with(
            "core: action_24_of_24: selected_0_0\n\
             core: replay_not_saved: saving_needs_file_io_in_bosonoga\n\
             core: action_24_of_24: quit\n"
        ),
        "{}",
        script.info
    );
}
