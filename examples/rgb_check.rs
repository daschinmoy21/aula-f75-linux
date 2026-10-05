//! Reversible Esc-green custom-light test. Restore with `--restore`.
use anyhow::{Result, ensure};
use aula_f75::devices::aula_f75::AulaF75;
use aula_f75::types::{Color, Key, KeyLayer};
use aula_f75::utils::extract_color_from_lights;
use std::path::PathBuf;

fn main() -> Result<()> {
    let restore = std::env::args().nth(1).as_deref() == Some("--restore");
    let d = AulaF75::new()?;
    let backup = PathBuf::from("/tmp/aula-f75-custom-check");
    if restore {
        let rgb = std::fs::read(backup.join("rgb.bin"))?;
        ensure!(rgb.len() == 384, "invalid custom-table backup");
        let keys: Vec<_> = (0..126)
            .map(|i| Key {
                light_pos: i,
                color: extract_color_from_lights(&rgb, i),
                ..Key::new_basic(format!("LED {i}"), "0x0".into(), i * 4, KeyLayer::Normal)
            })
            .collect();
        d.set_custom_light(&keys)?;
        d.set_basic_raw(&std::fs::read(backup.join("settings.bin"))?)?;
        println!("Restored custom table and lighting settings");
        return Ok(());
    }
    ensure!(
        !backup.exists(),
        "snapshot exists; use --restore before another test"
    );
    let rgb = d.get_custom_light()?;
    let settings = d.get_basic_raw()?;
    std::fs::create_dir_all(&backup)?;
    std::fs::write(backup.join("rgb.bin"), &rgb)?;
    std::fs::write(backup.join("settings.bin"), &settings)?;
    let key = Key {
        color: Color::create(0, 255, 0),
        ..Key::new_basic("Esc".into(), "0x29".into(), 0, KeyLayer::Normal)
    };
    d.set_custom_light(&[key])?;
    let mut expected = rgb;
    expected[0] = 0;
    expected[126] = 255;
    expected[252] = 0;
    ensure!(d.get_custom_light()? == expected, "custom RGB mismatch");
    let mut expected_settings = settings;
    expected_settings[9] = 1;
    expected_settings[10] = 21;
    ensure!(d.get_basic_raw()? == expected_settings, "settings mismatch");
    println!("Esc green stored, Custom enabled; all other table/settings bytes preserved.");
    println!("Restore: cargo run --example rgb_check -- --restore");
    Ok(())
}
