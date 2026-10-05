use anyhow::{Context, Result, anyhow};
use aula_f75::types::{Effect, KeyLayer};
use aula_f75::{connect, parse_profile, serialize_config};
use std::fs;

const USAGE: &str = "usage: aula-f75 <config.toml>                    apply keymap + lighting\n       aula-f75 --dump <names.toml> <out.toml>   read keymap + colours from the keyboard";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (dump, cfg_path) = match args.as_slice() {
        [f, p, _] if f == "--dump" => (true, p),
        [p] => (false, p),
        _ => return Err(anyhow!(USAGE)),
    };
    let profile =
        parse_profile(&fs::read_to_string(cfg_path).context("Failed to read config file")?)?;
    let keys = profile.keys;
    let device = connect()?;

    if dump {
        let read = device.fetch_keys_layer(KeyLayer::Normal, &[], &keys)?;
        fs::write(&args[2], serialize_config(&read)?).context("Failed to write dump")?;
        println!("Dumped keyboard state to {}", args[2]);
        return Ok(());
    }

    println!("Writing keymap...");
    device.set_keys(KeyLayer::Normal, &keys)?;
    device.set_custom_light(&keys)?;
    if let Some(id) = profile.effect {
        let effect = Effect::try_from(id).map_err(|_| anyhow!("Unsupported effect {id}"))?;
        if let Some(color) = profile.effect_color {
            device.set_effect_color(effect, &color)?;
        } else {
            device.set_light_mode(effect)?;
        }
    }
    println!("Done.");
    Ok(())
}
