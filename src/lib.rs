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
    /// Lighting effect id (profiles only; absent in plain keymap files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<u16>,
}

pub fn parse_config(s: &str) -> Result<Vec<Key>> {
    Ok(parse_profile(s)?.keys)
}

pub fn parse_profile(s: &str) -> Result<KeysWrapper> {
    toml::from_str::<KeysWrapper>(s).context("Failed to parse config")
}

pub fn serialize_config(keys: &[Key]) -> Result<String> {
    serialize_profile(keys, None)
}

pub fn serialize_profile(keys: &[Key], effect: Option<u16>) -> Result<String> {
    Ok(toml::to_string_pretty(&KeysWrapper {
        keys: keys.to_vec(),
        effect,
    })?)
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
    if api
        .device_list()
        .any(|d| d.vendor_id() == 0x3554 && d.product_id() == 0xfa09)
    {
        return Err(anyhow!(
            "Only the 2.4GHz dongle was found. Settings can't be changed wirelessly; plug in the USB cable."
        ));
    }
    Err(anyhow!(
        "No supported device found (USB connected? hidraw permissions?)"
    ))
}
