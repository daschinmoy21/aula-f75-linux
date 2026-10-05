use crate::types::{BatteryStatus, DeviceInfo, Effect, EffectColor, Key, KeyLayer, Macro};
use anyhow::Result;
use std::any::Any;

pub mod aula_f75;

pub trait DeviceDriver {
    fn matches(&self, vid: u16, pid: u16) -> bool;
    fn connect(&self, vid: u16, pid: u16) -> Result<Box<dyn Device>>;
}

pub trait Device: Any {
    fn get_uuid(&self) -> Result<u64>;
    fn fetch_battery(&self) -> Result<BatteryStatus>;

    // Expanded methods to support functionality in main.rs
    fn get_basic_info(&self) -> Result<DeviceInfo>;
    fn set_basic_info(&self, info: &DeviceInfo) -> Result<()>;
    fn set_light_mode(&self, effect: Effect) -> Result<()>;
    fn get_effect_color(&self, effect: Effect) -> Result<EffectColor>;
    fn set_effect_color(&self, effect: Effect, color: &EffectColor) -> Result<()>;
    fn get_keys(&self, layer: KeyLayer) -> Result<Vec<u8>>;
    fn set_keys(&self, layer: KeyLayer, keys: &[Key]) -> Result<()>;
    fn get_custom_light(&self) -> Result<Vec<u8>>;
    fn set_custom_light(&self, keys: &[Key]) -> Result<()>;
    fn fetch_custom_light(&self, keys: &mut [Key]) -> Result<()>;
    fn get_light_color(&self) -> Result<Vec<u8>>;
    fn set_light_color(&self) -> Result<()>;
    fn send_reset(&self) -> Result<()>;

    fn fetch_keys_layer(
        &self,
        layer: KeyLayer,
        macros: &[Macro],
        default_keys: &[Key],
    ) -> Result<Vec<Key>>;

    fn as_any(&self) -> &dyn Any;
}
