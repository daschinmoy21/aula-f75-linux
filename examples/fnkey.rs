// Sets ONLY the Fn key (matrix slot 53) in the Normal layer. Usage: fnkey <hex value>, e.g. 0x0d000001.
// Original value was 0x0d000000, so `fnkey 0x0d000000` puts it back.
use aula_f75::types::{Key, KeyLayer};
fn main() -> anyhow::Result<()> {
    let v = std::env::args().nth(1).expect("usage: fnkey 0x0d000001");
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    let mut k = Key::new_basic("Fn".into(), v.clone(), 53 * 4, KeyLayer::Normal);
    k.light_pos = 53;
    d.set_keys(KeyLayer::Normal, &[k])?;
    let raw = d.get_keys(KeyLayer::Normal)?;
    println!("slot 53 now {:02x?}", &raw[212..216]);
    Ok(())
}
