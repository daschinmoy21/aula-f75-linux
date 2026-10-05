// Steps through raw light-mode ids so you can see what each one really does on the keyboard.
// Writes only the mode bytes of the settings block (like the GUI's Apply) and restores your
// original mode on exit. Enter = next id, b = back, q = quit.
use std::io::{BufRead, Write};
fn main() -> anyhow::Result<()> {
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    let orig = d.get_basic_raw()?;
    let set = |id: u16| -> anyhow::Result<()> {
        let mut raw = d.get_basic_raw()?;
        raw[9..11].copy_from_slice(&id.to_be_bytes());
        d.set_basic_raw(&raw)
    };
    let max: u16 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(19);
    let stdin = std::io::stdin();
    let mut id = 0u16;
    loop {
        set(id)?;
        print!("mode id {id}: press keys, note what it does. [Enter=next, b=back, q=quit] ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 { break; }
        match line.trim() {
            "q" => break,
            "b" => id = id.saturating_sub(1),
            _ => { if id >= max { break } id += 1 }
        }
    }
    d.set_basic_raw(&orig)?;
    println!("restored original mode {}", u16::from_be_bytes([orig[9], orig[10]]));
    Ok(())
}
