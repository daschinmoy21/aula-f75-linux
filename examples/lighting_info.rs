// Read-only: inspect the active lighting mode and planar per-key RGB table.
use aula_f75::devices::aula_f75::AulaF75;
use aula_f75::utils::extract_color_from_lights;

fn main() -> anyhow::Result<()> {
    let d = AulaF75::new()?;
    let raw = d.get_basic_raw()?;
    let lights = d.get_custom_light()?;
    println!(
        "mode={} custom={} RGB bytes={}",
        raw[10],
        raw[9],
        lights.len()
    );
    for pos in [0, 8, 14, 35, 84] {
        let c = extract_color_from_lights(&lights, pos);
        println!("slot {pos}: RGB {},{},{}", c.r, c.g, c.b);
    }
    Ok(())
}
