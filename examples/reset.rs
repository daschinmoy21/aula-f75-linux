// Sends the keyboard's factory-reset command. Back up first (--dump, rawdump, layers).
fn main() -> anyhow::Result<()> {
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    d.send_reset()?;
    println!("reset sent");
    Ok(())
}
