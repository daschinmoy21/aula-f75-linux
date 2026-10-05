//! Backed-up check of Reaction's colour flag; restores with `--restore <snapshot>`.
use anyhow::{Result, ensure};
use aula_f75::devices::aula_f75::AulaF75;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let d = AulaF75::new()?;
    if args.first().is_some_and(|a| a == "--restore") {
        let path = args
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("missing snapshot path"))?;
        let raw = std::fs::read(path)?;
        d.set_basic_raw(&raw)?;
        ensure!(
            d.get_basic_raw()? == raw,
            "restored settings did not read back"
        );
        println!("Restored {path}");
        return Ok(());
    }
    let original = d.get_basic_raw()?;
    ensure!(
        original[10] == 12 && original[9] == 0,
        "expected active Reaction mode, got effect {} custom {}",
        original[10],
        original[9]
    );
    let offset = 56 + 12 * 2;
    println!(
        "Reaction: brightness={} speed={} colour-flag={}",
        original[offset],
        original[offset + 1] >> 4,
        original[offset + 1] & 15
    );
    let palette = d.get_light_color()?;
    println!(
        "Palette length={} first 36 bytes={:?}",
        palette.len(),
        &palette[..palette.len().min(36)]
    );
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let backup = PathBuf::from(format!("/tmp/aula-reaction-{stamp}"));
    std::fs::create_dir(&backup)?;
    std::fs::write(backup.join("settings.bin"), &original)?;
    std::fs::write(backup.join("palette.bin"), &palette)?;
    std::fs::write(backup.join("custom-rgb.bin"), d.get_custom_light()?)?;
    let mut raw = original.clone();
    raw[offset + 1] = (raw[offset + 1] & 0xf0) | 7;
    if raw[offset] == 0 {
        raw[offset] = 9;
    }
    d.set_basic_raw(&raw)?;
    if d.get_basic_raw()? != raw {
        d.set_basic_raw(&original)?;
        anyhow::bail!("settings did not read back; restored original");
    }
    println!("Reaction rainbow flag enabled; other bytes preserved.");
    println!(
        "Restore snapshot: {}",
        backup.join("settings.bin").display()
    );
    Ok(())
}
