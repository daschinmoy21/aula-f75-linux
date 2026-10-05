// Read-only: print the raw 128-byte settings block, 16 bytes per row.
fn main() -> anyhow::Result<()> {
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    let raw = d.get_basic_raw()?;
    for (i, row) in raw.chunks(16).enumerate() {
        println!("{:3}: {}", i * 16, row.iter().map(|b| format!("{b:3}")).collect::<Vec<_>>().join(" "));
    }
    Ok(())
}
