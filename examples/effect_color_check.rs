//! Test a global red Reaction colour without changing the custom per-key RGB table.
use anyhow::{Result, ensure};
use aula_f75::devices::aula_f75::AulaF75;
use aula_f75::types::{Color, Effect, EffectColor};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> Result<()> {
    let d = AulaF75::new()?;
    let raw = d.get_basic_raw()?;
    ensure!(raw[10] == 12, "Select Reaction first");
    let palette = d.get_light_color()?;
    let rgb = d.get_custom_light()?;
    let backup = PathBuf::from(format!(
        "/tmp/aula-effect-color-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    std::fs::create_dir(&backup)?;
    std::fs::write(backup.join("settings.bin"), &raw)?;
    std::fs::write(backup.join("palette.bin"), &palette)?;
    std::fs::write(backup.join("custom-rgb.bin"), &rgb)?;
    d.set_effect_color(
        Effect::Reaction,
        &EffectColor {
            color: Color::create(255, 0, 0),
            rainbow: false,
        },
    )?;
    ensure!(d.get_custom_light()? == rgb, "custom per-key RGB changed");
    println!("Reaction global red applied and read back; custom per-key table preserved.");
    println!("Snapshot: {}", backup.display());
    Ok(())
}
