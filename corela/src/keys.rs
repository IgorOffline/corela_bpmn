use macroquad::miniquad::KeyCode;

// The names of bosonoga's keys are sokol_app's, and miniquad's are its
// variants in CamelCase, as KeyCode::PageUp for SAPP_KEYCODE_PAGE_UP.
pub(crate) fn code(keycode: KeyCode) -> Option<u32> {
    let variant = format!("{keycode:?}");
    let word = variant.strip_prefix("Key").unwrap_or(&variant);
    let mut name = String::from("SAPP_KEYCODE_");
    let mut previous: Option<char> = None;
    for letter in word.chars() {
        let starts_word = letter.is_ascii_uppercase()
            || (letter.is_ascii_digit() && previous.is_some_and(|p| p.is_ascii_lowercase()));
        if starts_word && previous.is_some() {
            name.push('_');
        }
        name.push(letter.to_ascii_uppercase());
        previous = Some(letter);
    }
    bosonoga::key::named(&name)
}
