//! Validate the complete configuration before any device mutation.
use crate::types::{Key, KeyType};
use crate::utils::parse_hex;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;

// The F75 has 126 keymap slots and 126 LED indices in three RGB planes.
pub(crate) const KEYMAP_BYTES: usize = 504;
pub(crate) const LIGHT_SLOTS: usize = 126;
pub(crate) const LIGHT_BYTES: usize = LIGHT_SLOTS * 3 + 6;

pub fn validate_keys(keys: &[Key]) -> Result<()> {
    let mut slots = HashSet::new();
    let mut lights = HashSet::new();
    for (i, key) in keys.iter().enumerate() {
        let validate = || -> Result<()> {
            ensure!(
                key.key_type == KeyType::Basic && key.macro_data.is_none(),
                "Macros are not supported (key_type must be basic and macro_data must be absent)"
            );
            parse_hex(&key.value)?;
            ensure!(
                key.pos < KEYMAP_BYTES && key.pos % 4 == 0,
                "Invalid key position {}: expected a multiple of 4 in 0..={}",
                key.pos,
                KEYMAP_BYTES - 4
            );
            ensure!(
                key.light_pos < LIGHT_SLOTS,
                "Invalid lighting position {}: expected 0..={}",
                key.light_pos,
                LIGHT_SLOTS - 1
            );
            Ok(())
        };
        validate().with_context(|| {
            format!("Key {} ({:?}, layer {})", i + 1, key.name, key.layer as u8)
        })?;
        ensure!(
            slots.insert((key.layer as u8, key.pos)),
            "Duplicate key position {} in layer {} ({:?})",
            key.pos,
            key.layer as u8,
            key.name
        );
        ensure!(
            lights.insert((key.layer as u8, key.light_pos)),
            "Duplicate lighting position {} in layer {} ({:?})",
            key.light_pos,
            key.layer as u8,
            key.name
        );
    }
    Ok(())
}
