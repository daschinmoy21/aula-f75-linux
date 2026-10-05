// Read-only: print every raw input report from all interfaces of the keyboard (258a:010c) for N seconds.
use hidapi::HidApi;
use std::time::{Duration, Instant};
fn main() -> anyhow::Result<()> {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let api = HidApi::new()?;
    let mut devs = Vec::new();
    for info in api.device_list().filter(|d| d.vendor_id() == 0x258a && d.product_id() == 0x010c) {
        // one handle per hidraw node
        if devs.iter().any(|(p, _): &(String, _)| *p == info.path().to_string_lossy()) { continue; }
        if let Ok(d) = info.open_device(&api) {
            d.set_blocking_mode(false)?;
            devs.push((info.path().to_string_lossy().to_string(), d));
        }
    }
    let t0 = Instant::now();
    let mut buf = [0u8; 64];
    while t0.elapsed() < Duration::from_secs(secs) {
        for (p, d) in &devs {
            let n = d.read(&mut buf)?;
            if n > 0 {
                println!("{:6.2}s {} {:02x?}", t0.elapsed().as_secs_f32(), p.rsplit('/').next().unwrap_or(p), &buf[..n]);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
