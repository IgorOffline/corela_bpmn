use bosonoga::gfx::{Horizontal, Scene, Shape, Vertical};
use macroquad::color::Color;
use macroquad::shapes::{draw_poly, draw_rectangle};
use macroquad::text::{Font, TextParams, draw_text_ex, load_ttf_font_from_bytes, measure_text};
use macroquad::window::clear_background;

const HEX_CORNERS: u8 = 6;
// Enough that a circle of corela's size looks round.
const CIRCLE_CORNERS: u8 = 48;

struct Typeface {
    font: Font,
    // Of the em, which fontdue sizes by, to the height from the descender to
    // the ascender, which fontstash sizes by.
    em_per_height: f32,
    ascender: f32,
    descender: f32,
}

pub(crate) struct Fonts {
    typefaces: Vec<Typeface>,
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
        *bytes.get(at + 2)?,
        *bytes.get(at + 3)?,
    ]))
}

fn table(bytes: &[u8], tag: [u8; 4]) -> Option<usize> {
    let count = usize::from(read_u16(bytes, 4)?);
    (0..count)
        .map(|i| 12 + 16 * i)
        .find(|entry| bytes.get(*entry..entry + 4) == Some(tag.as_slice()))
        .and_then(|entry| usize::try_from(read_u32(bytes, entry + 8)?).ok())
}

// The units per em of head, and the ascender and descender of hhea.
fn metrics(bytes: &[u8]) -> Option<(f32, f32, f32)> {
    let head = table(bytes, *b"head")?;
    let hhea = table(bytes, *b"hhea")?;
    let units = f32::from(read_u16(bytes, head + 18)?);
    let ascender = f32::from(i16::from_be_bytes(read_u16(bytes, hhea + 4)?.to_be_bytes()));
    let descender = f32::from(i16::from_be_bytes(read_u16(bytes, hhea + 6)?.to_be_bytes()));
    (units > 0.0 && ascender > descender).then_some((units, ascender, descender))
}

impl Fonts {
    pub(crate) fn load(
        program: &str,
        files: &[(String, String, Vec<u8>)],
    ) -> Result<Fonts, String> {
        let mut typefaces = Vec::new();
        for (key, path, bytes) in files {
            let not_truetype = || format!("{program}: font_{key}: {path} is not a TrueType font");
            let (units, ascender, descender) = metrics(bytes).ok_or_else(not_truetype)?;
            let font = load_ttf_font_from_bytes(bytes).map_err(|_| not_truetype())?;
            let height = ascender - descender;
            typefaces.push(Typeface {
                font,
                em_per_height: units / height,
                ascender: ascender / height,
                descender: descender / height,
            });
        }
        Ok(Fonts { typefaces })
    }
}

fn color(rgb: u32) -> Color {
    let [_, red, green, blue] = rgb.to_be_bytes();
    Color::from_rgba(red, green, blue, u8::MAX)
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the size is rounded, positive and far below u16::MAX"
)]
fn font_size(pixels: f32) -> u16 {
    pixels.round().clamp(1.0, f32::from(u16::MAX)) as u16
}

pub(crate) fn scene(scene: &Scene, fonts: &Fonts) {
    clear_background(color(scene.clear));
    for shape in &scene.shapes {
        match shape {
            Shape::Rectangle {
                x,
                y,
                width,
                height,
                color: rgb,
            } => draw_rectangle(*x, *y, *width, *height, color(*rgb)),
            Shape::Hex {
                x,
                y,
                radius,
                color: rgb,
            } => draw_poly(*x, *y, HEX_CORNERS, *radius, 0.0, color(*rgb)),
            Shape::Circle {
                x,
                y,
                radius,
                color: rgb,
            } => draw_poly(*x, *y, CIRCLE_CORNERS, *radius, 0.0, color(*rgb)),
            Shape::Text {
                x,
                y,
                font,
                size,
                color: rgb,
                align,
                text,
            } => {
                let Some(typeface) = fonts.typefaces.get(*font) else {
                    continue;
                };
                let em = size * typeface.em_per_height;
                let rasterized = font_size(em);
                let scale = em / f32::from(rasterized);
                let width = measure_text(text, Some(&typeface.font), rasterized, scale).width;
                let left = match align.horizontal {
                    Horizontal::Left => *x,
                    Horizontal::Center => x - width / 2.0,
                    Horizontal::Right => x - width,
                };
                let baseline = match align.vertical {
                    Vertical::Baseline => *y,
                    Vertical::Top => y + size * typeface.ascender,
                    Vertical::Middle => y + size * (typeface.ascender + typeface.descender) / 2.0,
                    Vertical::Bottom => y + size * typeface.descender,
                };
                draw_text_ex(
                    text,
                    left,
                    baseline,
                    TextParams {
                        font: Some(&typeface.font),
                        font_size: rasterized,
                        font_scale: scale,
                        color: color(*rgb),
                        ..TextParams::default()
                    },
                );
            }
        }
    }
}
