//! AULA F75 driver library: HID protocol, key/light types, config (de)serialisation.
pub mod devices;
pub mod types;
pub mod utils;

use anyhow::{Context, Result, anyhow};
use devices::{Device, DeviceDriver, aula_f75::AulaF75Driver};
use types::Key;

/// Stock keymap + colours; also the source of key names when reading from the keyboard.
pub const DEFAULT_CONFIG: &str = include_str!("../data/default.toml");

#[derive(serde::Deserialize, serde::Serialize)]
pub struct KeysWrapper {
    pub keys: Vec<Key>,
}

pub fn parse_config(s: &str) -> Result<Vec<Key>> {
    Ok(toml::from_str::<KeysWrapper>(s).context("Failed to parse config")?.keys)
}

pub fn serialize_config(keys: &[Key]) -> Result<String> {
    Ok(toml::to_string_pretty(&KeysWrapper { keys: keys.to_vec() })?)
}

/// Open the first supported keyboard that answers on its vendor HID interface.
pub fn connect() -> Result<Box<dyn Device>> {
    let drivers: Vec<Box<dyn DeviceDriver>> = vec![Box::new(AulaF75Driver)];
    let api = hidapi::HidApi::new().context("Failed to initialize HID API")?;
    for info in api.device_list() {
        for d in &drivers {
            if d.matches(info.vendor_id(), info.product_id()) {
                if let Ok(dev) = d.connect(info.vendor_id(), info.product_id()) {
                    if dev.get_uuid().is_ok() {
                        return Ok(dev);
                    }
                }
            }
        }
    }
    Err(anyhow!("No supported device found (USB connected? hidraw permissions?)"))
}
