use std::path::Path;

use bosonoga::ast::ExitStatus;
use bosonoga::gfx::{Input, Scene, Shape, Window};
use bosonoga::io::{FileIo, Io};
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
        let keys = match self.keys.get(index) {
            Some(keys) => keys.clone(),
            None => Vec::new(),
        };
        Some(Input {
            keys,
            delta_time: 1.0 / 60.0,
        })
    }
}

#[cfg(test)]
fn run(keys: &[&[&str]], closes: bool) -> (Result<ExitStatus, String>, Script) {
    run_with_io(keys, closes, &mut FileIo)
}

#[cfg(test)]
fn run_with_io(
    keys: &[&[&str]],
    closes: bool,
    io: &mut dyn Io,
) -> (Result<ExitStatus, String>, Script) {
    let settings = Settings::read(Path::new("regina.ini")).unwrap();
    let program = bosonoga::load(Path::new(PROGRAM), &settings).unwrap();
    let mut codes = Vec::new();
    for frame in keys {
        let mut frame_codes = Vec::new();
        for name in *frame {
            frame_codes.push(bosonoga::key::named(name).unwrap());
        }
        codes.push(frame_codes);
    }
    let mut script = Script {
        keys: codes,
        closes,
        ..Script::default()
    };
    let result = bosonoga::gfx::run_with_io(&program, &mut script, io);
    (result, script)
}

#[cfg(test)]
fn hexes(scene: &Scene, color: u32) -> Vec<(f32, f32)> {
    let mut found = Vec::new();
    for shape in &scene.shapes {
        if let Shape::Hex { x, y, color: c, .. } = shape
            && *c == color
        {
            found.push((*x, *y));
        }
    }
    found
}

#[cfg(test)]
fn texts(scene: &Scene, color: u32) -> Vec<&str> {
    let mut found = Vec::new();
    for shape in &scene.shapes {
        if let Shape::Text { text, color: c, .. } = shape
            && *c == color
        {
            found.push(text.as_str());
        }
    }
    found
}

#[cfg(test)]
fn circles(scene: &Scene) -> Vec<(f32, f32, f32, u32)> {
    let mut found = Vec::new();
    for shape in &scene.shapes {
        if let Shape::Circle {
            x,
            y,
            radius,
            color,
        } = shape
        {
            found.push((*x, *y, *radius, *color));
        }
    }
    found
}

#[cfg(test)]
fn icons(scene: &Scene) -> Vec<(f32, f32, usize, f32, u32)> {
    let mut found = Vec::new();
    for shape in &scene.shapes {
        if let Shape::Icon {
            x,
            y,
            icon,
            angle,
            color,
        } = shape
        {
            found.push((*x, *y, *icon, *angle, *color));
        }
    }
    found
}

#[cfg(test)]
fn has_circle_of_color(scene: &Scene, color: u32) -> bool {
    for (_, _, _, circle_color) in circles(scene) {
        if circle_color == color {
            return true;
        }
    }
    false
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
    assert_eq!(
        circles(last),
        [
            (576.0, 324.0 + 4.0 * 32.0 * 1.732_050_8, 15.75, 0xE7_8284),
            (576.0, 324.0 + 4.0 * 32.0 * 1.732_050_8, 14.25, 0xC6_D0F5),
        ]
    );
    let arrows = icons(last);
    let [(x, y, 0, angle, 0x30_3446)] = arrows[..] else {
        panic!("one selected arrow, not {arrows:?}");
    };
    assert!(near((x, y), selected) && angle == 0.0, "{arrows:?}");
}

#[test]
fn the_last_action_ends_the_run() {
    let mut keys: Vec<&[&str]> = vec![&["SAPP_KEYCODE_I"]];
    for _ in 0..12 {
        keys.push(&["SAPP_KEYCODE_W"]);
        keys.push(&["SAPP_KEYCODE_S"]);
    }
    let mut io = TestIo::default();
    let (result, script) = run_with_io(&keys, true, &mut io);
    assert_eq!(result, Ok(ExitStatus::Success));
    assert_eq!(io.files.len(), 1);
    assert!(
        script.info.ends_with(
            "core: action_24_of_24: selected_0_0\n\
             core: replay_saved: replay_1000.xml\n\
             core: action_24_of_24: quit\n"
        ),
        "{}",
        script.info
    );
}

#[derive(Default)]
struct TestIo {
    files: std::collections::BTreeMap<String, String>,
    fail_next_write: bool,
}

impl Io for TestIo {
    fn read(&mut self, _: &Path, name: &str) -> std::io::Result<String> {
        match self.files.get(name) {
            Some(text) => Ok(text.clone()),
            None => Err(std::io::ErrorKind::NotFound.into()),
        }
    }

    fn write(&mut self, _: &Path, name: &str, text: &str) -> std::io::Result<()> {
        use std::collections::btree_map::Entry;
        if std::mem::take(&mut self.fail_next_write) {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        if let Entry::Vacant(entry) = self.files.entry(name.to_owned()) {
            entry.insert(text.to_owned());
            Ok(())
        } else {
            Err(std::io::ErrorKind::AlreadyExists.into())
        }
    }

    fn modified(&mut self, _: &Path, _: &str) -> std::io::Result<std::time::SystemTime> {
        unreachable!("Corela does not query modification times directly")
    }

    fn latest(&mut self, _: &Path, prefix: &str, suffix: &str) -> std::io::Result<String> {
        let mut latest = None;
        for name in self.files.keys() {
            if name.starts_with(prefix) && name.ends_with(suffix) {
                latest = Some(name.clone());
            }
        }
        match latest {
            Some(name) => Ok(name),
            None => Err(std::io::ErrorKind::NotFound.into()),
        }
    }
}

#[test]
fn replay_keeps_the_undone_branch() {
    let cycle = "SAPP_KEYCODE_I";
    let run_key = "SAPP_KEYCODE_R";
    let mut io = TestIo::default();
    let (_, recorded) = run_with_io(
        &[
            &[cycle, "SAPP_KEYCODE_1"],
            &[cycle, run_key],
            &["SAPP_KEYCODE_ESCAPE", cycle, "SAPP_KEYCODE_0"],
            &[cycle, cycle, cycle, run_key],
        ],
        true,
        &mut io,
    );
    assert!(
        recorded.info.contains("replay_saved: replay_1000.xml"),
        "{}",
        recorded.info
    );

    let (result, playback) = run_with_io(
        &[&[cycle, cycle, cycle, cycle, cycle, run_key]],
        false,
        &mut io,
    );
    assert_eq!(result, Ok(ExitStatus::Success));
    assert!(
        playback.info.contains(
            "core: replay_action_1_of_3: placed_arrow_at_0_0\n\
         core: replay_action_2_of_3: undo_0_more_to_undo_1_to_redo\n\
         core: replay_action_3_of_3: placed_start_node_at_0_0\n\
         core: replay_finished\n"
        ),
        "{}",
        playback.info
    );
    let icon_counts = [
        icons(&playback.frames[10]).len(),
        icons(&playback.frames[40]).len(),
        icons(&playback.frames[70]).len(),
    ];
    assert_eq!(icon_counts, [1, 0, 0]);
    assert!(has_circle_of_color(&playback.frames[70], 0xCD_D6F4));
}

#[test]
fn failed_final_save_can_be_retried() {
    let mut keys: Vec<&[&str]> = vec![&["SAPP_KEYCODE_I"]];
    for _ in 0..12 {
        keys.push(&["SAPP_KEYCODE_W"]);
        keys.push(&["SAPP_KEYCODE_S"]);
    }
    keys.push(&["SAPP_KEYCODE_R"]);
    let mut io = TestIo {
        fail_next_write: true,
        ..TestIo::default()
    };
    let (result, script) = run_with_io(&keys, true, &mut io);
    assert_eq!(result, Ok(ExitStatus::Success));
    assert!(script.info.contains("core: replay_not_saved:"));
    assert!(
        script
            .info
            .ends_with("core: replay_saved: replay_1000.xml\ncore: action_24_of_24: quit\n")
    );
    assert_eq!(io.files.len(), 1);
}

#[test]
fn malformed_replay_leaves_the_board_intact() {
    let cycle = "SAPP_KEYCODE_I";
    let mut io = TestIo::default();
    let _ = run_with_io(
        &[
            &[cycle, "SAPP_KEYCODE_1", "SAPP_KEYCODE_1"],
            &[cycle, cycle, cycle, "SAPP_KEYCODE_R"],
        ],
        true,
        &mut io,
    );
    let replay = io.files.get_mut("replay_1000.xml").unwrap();
    *replay = replay.replace("number='2'", "number='3'");
    let (result, script) = run_with_io(
        &[
            &[cycle, "SAPP_KEYCODE_1"],
            &[cycle, cycle, cycle, cycle, "SAPP_KEYCODE_R"],
            &[],
        ],
        true,
        &mut io,
    );
    assert!(result.is_err());
    assert!(
        script
            .info
            .contains("core: replay_not_loaded: invalid replay")
    );
    let last = script.frames.last().unwrap();
    assert_ne!(icons(last).len(), 0);
}
