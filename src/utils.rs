use crate::types::{Color, Key};
use std::collections::HashMap;

pub fn parse_hex(s: &str) -> anyhow::Result<u32> {
    let digits = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    anyhow::ensure!(
        !digits.is_empty() && digits.len() <= 8 && digits.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid hexadecimal key value {s:?}: expected 1–8 hex digits, optionally prefixed with 0x"
    );
    Ok(u32::from_str_radix(digits, 16)?)
}

pub fn build_key_lookup(keys: &[Key]) -> HashMap<String, String> {
    keys.iter()
        .map(|key| (key.value.clone(), key.name.clone()))
        .collect()
}

pub fn extract_color_from_lights(lights: &[u8], pos: usize) -> Color {
    if pos >= 126 || lights.len() < 378 {
        return Color::default();
    }
    Color::create(lights[pos], lights[pos + 126], lights[pos + 252])
}
