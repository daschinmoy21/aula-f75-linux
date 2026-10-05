// Patch the settings block from a known-good dump (rawdump output of a healthy F75).
//   blockpatch <good.txt>             show what would change (dry run)
//   blockpatch <good.txt> --yes       copy bytes [0..9) and [56..128) from the good dump into this keyboard
//   add --head to patch only bytes [0..9) (mac flag, polling, ...) and leave the lighting table alone
//   blockpatch <saved.txt> --restore  write a saved dump back verbatim (rollback)
// The mode ([9..11]), sleep and everything in [9..56) are left as they are on the patched keyboard.
use anyhow::{Result, bail};
fn parse(path: &str) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    for l in std::fs::read_to_string(path)?.lines() {
        if let Some((_, rest)) = l.split_once(':') {
            for t in rest.split_whitespace() { v.push(t.parse::<u8>()?); }
        }
    }
    if v.len() != 128 { bail!("{path}: expected 128 bytes, got {}", v.len()); }
    Ok(v)
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let good = parse(args.first().map(String::as_str).unwrap_or_else(|| { eprintln!("usage: blockpatch <dump.txt> [--yes|--restore]"); std::process::exit(2) }))?;
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    let cur = d.get_basic_raw()?;
    let mut new = cur.clone();
    if args.iter().any(|a| a == "--restore") {
        new = good.clone();
    } else {
        new[0..9].copy_from_slice(&good[0..9]);
        if !args.iter().any(|a| a == "--head") {
            new[56..128].copy_from_slice(&good[56..128]);
        }
    }
    let diffs: Vec<String> = (0..128).filter(|&i| cur[i] != new[i]).map(|i| format!("[{i}] {}->{}", cur[i], new[i])).collect();
    println!("{} byte(s) would change:\n{}", diffs.len(), diffs.join(" "));
    if args.iter().any(|a| a == "--yes" || a == "--restore") {
        d.set_basic_raw(&new)?;
        std::thread::sleep(std::time::Duration::from_millis(300));
        let back = d.get_basic_raw()?;
        println!("written; read back {} differing byte(s) vs requested", (0..128).filter(|&i| back[i] != new[i]).count());
    } else {
        println!("(dry run, nothing written; add --yes to write)");
    }
    Ok(())
}
