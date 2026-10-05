use anyhow::{Context, Result, anyhow};
use aula_f75::types::{Effect, KeyLayer};
use aula_f75::{connect, parse_config, serialize_config};
use std::fs;

const USAGE: &str = "usage: aula-f75 <config.toml>                    apply keymap + lighting\n       aula-f75 --dump <names.toml> <out.toml>   read keymap + colours from the keyboard";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dump, cfg_path) = match args.as_slice() {
        [f, p, _] if f == "--dump" => (true, p),
        [p] => (false, p),
        _ => return Err(anyhow!(USAGE)),
    };
    let keys = parse_config(&fs::read_to_string(cfg_path).context("Failed to read config file")?)?;
    let device = connect()?;

    if dump {
        let read = device.fetch_keys_layer(KeyLayer::Normal, &[], &keys)?;
        fs::write(&args[2], serialize_config(&read)?).context("Failed to write dump")?;
        println!("Dumped keyboard state to {}", args[2]);
        return Ok(());
    }

    println!("Writing keymap...");
    device.set_keys(KeyLayer::Normal, &keys)?;
    let mut info = device.get_basic_info()?;
    info.light_mode = Effect::FixedOn;
    device.set_basic_info(&info)?;
    device.set_custom_light(&keys)?;
    device.set_light_color()?;
    println!("Done.");
    Ok(())
}
