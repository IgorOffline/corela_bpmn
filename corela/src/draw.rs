use bosonoga::gfx::{Horizontal, Scene, Shape, Vertical};
use macroquad::color::Color;
use macroquad::prelude::ImageFormat;
use macroquad::shapes::{draw_poly, draw_rectangle};
use macroquad::text::{Font, TextParams, draw_text_ex, load_ttf_font_from_bytes, measure_text};
use macroquad::texture::{DrawTextureParams, Image, Texture2D, draw_texture_ex};
use macroquad::window::clear_background;

const HEX_CORNERS: u8 = 6;
const ICON_SIDE_MAX: u32 = 1024;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
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

pub(crate) struct Assets {
    typefaces: Vec<Typeface>,
    icons: Vec<Texture2D>,
}

pub(crate) type Files = [(String, String, Vec<u8>)];

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

// The width and the height of the header of a PNG, which is checked first so
// that a large one is not decoded.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    (bytes.get(..8)? == PNG_SIGNATURE.as_slice() && bytes.get(12..16)? == b"IHDR".as_slice())
        .then_some(())?;
    Some((read_u32(bytes, 16)?, read_u32(bytes, 20)?))
}

// Only its alpha is kept, over white, so that the color of gfx_draw_icon tints
// it without dark edges where it is filtered.
fn icon(program: &str, key: &str, path: &str, bytes: &[u8]) -> Result<Texture2D, String> {
    let prefix = format!("{program}: icon_{key}: {path}");
    let (width, height) = png_size(bytes).ok_or_else(|| format!("{prefix} is not a PNG image"))?;
    if width > ICON_SIDE_MAX || height > ICON_SIDE_MAX {
        return Err(format!(
            "{prefix} is {width} by {height} pixels, more than {ICON_SIDE_MAX} a side"
        ));
    }
    let mut image = Image::from_file_with_format(bytes, Some(ImageFormat::Png))
        .map_err(|error| format!("{prefix} is not a PNG image: {error}"))?;
    for [red, green, blue, _] in image.bytes.as_chunks_mut::<4>().0 {
        *red = u8::MAX;
        *green = u8::MAX;
        *blue = u8::MAX;
    }
    Ok(Texture2D::from_image(&image))
}

impl Assets {
    pub(crate) fn load(program: &str, fonts: &Files, icons: &Files) -> Result<Assets, String> {
        let mut typefaces = Vec::new();
        for (key, path, bytes) in fonts {
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
        let mut textures = Vec::new();
        for (key, path, bytes) in icons {
            textures.push(icon(program, key, path, bytes)?);
        }
        Ok(Assets {
            typefaces,
            icons: textures,
        })
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

pub(crate) fn scene(scene: &Scene, assets: &Assets) {
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
            Shape::Icon {
                x,
                y,
                icon,
                angle,
                color: rgb,
            } => {
                let Some(texture) = assets.icons.get(*icon) else {
                    continue;
                };
                let half_width = texture.width() / 2.0;
                let half_height = texture.height() / 2.0;
                draw_texture_ex(
                    texture,
                    (x - half_width).round(),
                    (y - half_height).round(),
                    color(*rgb),
                    DrawTextureParams {
                        rotation: *angle,
                        ..DrawTextureParams::default()
                    },
                );
            }
            Shape::Text {
                x,
                y,
                font,
                size,
                color: rgb,
                align,
                text,
            } => {
                let Some(typeface) = assets.typefaces.get(*font) else {
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
