fn main() -> anyhow::Result<()> {
    let d = aula_f75::connect()?;
    let i = d.get_basic_info()?;
    println!("mode={:?}\n{:?}", i.light_mode, i);
    Ok(())
}
