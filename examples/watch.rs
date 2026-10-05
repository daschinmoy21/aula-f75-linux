// Read-only: poll the settings block and print every change (mode id + changed byte indices).
use std::{thread::sleep, time::{Duration, Instant}};
fn main() -> anyhow::Result<()> {
    let d = aula_f75::devices::aula_f75::AulaF75::new()?;
    let t0 = Instant::now();
    let mut prev: Vec<u8> = Vec::new();
    loop {
        let raw = d.get_basic_raw()?;
        if raw != prev {
            let diffs: Vec<String> = if prev.is_empty() {
                vec![]
            } else {
                (0..raw.len()).filter(|&i| raw[i] != prev[i]).map(|i| format!("[{i}] {}->{}", prev[i], raw[i])).collect()
            };
            println!("{:6.1}s mode={} {}", t0.elapsed().as_secs_f32(), u16::from_be_bytes([raw[9], raw[10]]), diffs.join(" "));
            prev = raw;
        }
        sleep(Duration::from_millis(250));
    }
}
