// Read-only: print non-zero key codes of each layer id (0..8) by matrix slot.
fn main() -> anyhow::Result<()> {
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    for layer in 0u8..8 {
        let raw = match d.get_keys_raw_layer(layer) { Ok(r) => r, Err(e) => { println!("layer {layer}: error {e}"); continue } };
        let ents: Vec<String> = raw.chunks(4).enumerate()
            .filter(|(_, c)| c.iter().any(|&b| b != 0))
            .map(|(i, c)| format!("{i}:{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])).collect();
        println!("layer {layer}: {} entries\n  {}", ents.len(), ents.join(" "));
    }
    Ok(())
}
