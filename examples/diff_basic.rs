// Read-only: compare the keyboard's real settings block with what the legacy writer would send.
use aula_f75::devices::aula_f75::AulaF75;
fn main() -> anyhow::Result<()> {
    let d = AulaF75::new()?;
    let raw = d.get_basic_raw()?;
    let info = d.get_basic_info()?;
    let legacy = AulaF75::legacy_basic_payload(&info);
    println!("idx  device  legacy_write");
    for i in 0..raw.len() {
        let w = legacy[7 + i];
        if raw[i] != w {
            println!("{:3}  {:5}  {:5}", i, raw[i], w);
        }
    }
    println!("raw: {:?}", raw);
    Ok(())
}
