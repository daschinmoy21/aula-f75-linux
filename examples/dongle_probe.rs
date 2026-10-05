// Read-only: send known GET commands to the 2.4GHz dongle (3554:FA09) and dump whatever comes back.
use hidapi::HidApi;
fn main() -> anyhow::Result<()> {
    let api = HidApi::new()?;
    for info in api.device_list().filter(|d| d.vendor_id() == 0x3554 && d.product_id() == 0xfa09) {
        println!("iface {} usage_page {:#06x} usage {:#x} {}", info.interface_number(), info.usage_page(), info.usage(), info.path().to_string_lossy());
        if info.usage_page() != 0xff02 && info.usage_page() != 0xff04 { continue; }
        let dev = info.open_device(&api)?;
        // (name, 7-byte command) -- all GETs from the wired protocol
        let cmds: [(&str, [u8; 7]); 3] = [
            ("uuid", [130, 1, 0, 1, 0, 6, 0]),
            ("battery", [135, 0, 0, 1, 0, 2, 0]),
            ("basic-info", [132, 0, 0, 1, 0, 128, 0]),
        ];
        for (name, c) in cmds {
            let mut tx = vec![6u8];
            tx.extend_from_slice(&c);
            println!("-> {name}: send_feature {:02x?} = {:?}", tx, dev.send_feature_report(&tx));
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut rx = vec![0u8; 64]; rx[0] = 6;
            println!("   get_feature: {:?} {:02x?}", dev.get_feature_report(&mut rx), rx);
            let mut inb = [0u8; 64];
            let n = dev.read_timeout(&mut inb, 300);
            println!("   input: {:?} {:02x?}", n, &inb[..n.as_ref().map(|n| *n).unwrap_or(0)]);
        }
    }
    Ok(())
}
