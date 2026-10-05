use crate::types::{BatteryStatus, DeviceInfo, Effect, Key, KeyLayer, KeyType, LightParam, Macro};
use crate::utils::{build_key_lookup, extract_color_from_lights, parse_hex};
use anyhow::{Context, Result, anyhow, bail};
use hidapi::{HidApi, HidDevice};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// ============================================================================
// Constants
// ============================================================================

pub const VENDOR_ID: u16 = 9610;
pub const PRODUCT_ID: u16 = 268;
const REPORT_ID_TX: u8 = 6;
const REPORT_ID_RX: u8 = 6;
const SEND_PAYLOAD_LENGTH: usize = 519;
const LIGHT_PARAMS_LENGTH: usize = 34;
const LIGHT_OFFSET: usize = 65;
const PAYLOAD_LENGTH_BASIC_INFO: usize = 128;
const PAYLOAD_LENGTH_KEYS: usize = 512;
const RESET_DELAY_MS: u64 = 2000;

// Command constants
const CMD_GET_UUID: [u8; 7] = [130, 1, 0, 1, 0, 6, 0];
const CMD_RESET: [u8; 7] = [17, 0, 0, 1, 0, 1, 0];
const CMD_GET_BASIC_INFO: [u8; 6] = [132, 0, 0, 1, 0, 128];
const CMD_GET_BATTERY: [u8; 7] = [135, 0, 0, 1, 0, 2, 0];
const CMD_GET_KEYS: [u8; 7] = [131, 0, 0, 1, 0, 248, 1];
const CMD_SET_BASIC_INFO: [u8; 6] = [4, 0, 0, 1, 0, 128];
const CMD_SET_KEY: [u8; 7] = [3, 0, 0, 1, 0, 248, 1];
const CMD_SEND_MACRO: [u8; 7] = [5, 0, 0, 1, 0, 0, 0];
const CMD_GET_CUSTOM_LIGHT: [u8; 7] = [134, 0, 0, 1, 0, 128, 1];
const CMD_SET_CUSTOM_LIGHT: [u8; 7] = [6, 0, 0, 1, 0, 128, 1];
const CMD_GET_LIGHT_COLOR: [u8; 7] = [138, 0, 0, 1, 0, 128, 2];
const CMD_SET_LIGHT_COLOR: [u8; 7] = [10, 0, 0, 1, 0, 0, 2];

const SL: [u8; 365] = [
    255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0,
    0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0,
    255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0,
    0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0,
    255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255,
    255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255,
    255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0,
    255, 0, 255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0,
    255, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0,
    255, 255, 255, 255, 255, 208, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255,
    255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255,
    255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255,
    255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255,
    255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255,
    255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 255, 255, 3, 0, 0,
    0, 0, 119, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90, 165, 0, 0, 0, 0,
];

const MAGIC_INDEXES: [usize; 18] = [
    66, 68, 70, 72, 74, 76, 78, 80, 82, 84, 86, 88, 90, 92, 94, 96, 98, 100,
];

// ============================================================================
// Driver Struct
// ============================================================================

pub struct AulaF75Driver;

impl super::DeviceDriver for AulaF75Driver {
    fn matches(&self, vid: u16, pid: u16) -> bool {
        vid == VENDOR_ID && pid == PRODUCT_ID
    }

    fn connect(&self, vid: u16, pid: u16) -> Result<Box<dyn super::Device>> {
        if !self.matches(vid, pid) {
            bail!("Device VID/PID mismatch");
        }
        let device = AulaF75::new()?;
        Ok(Box::new(device))
    }
}

pub struct AulaF75 {
    device: Mutex<HidDevice>,
}

impl super::Device for AulaF75 {
    fn get_uuid(&self) -> Result<u64> {
        self.get_uuid()
    }

    fn fetch_battery(&self) -> Result<BatteryStatus> {
        self.fetch_battery()
    }

    fn get_basic_info(&self) -> Result<DeviceInfo> {
        self.get_basic_info()
    }

    fn set_basic_info(&self, info: &DeviceInfo) -> Result<()> {
        self.set_basic_info(info)
    }

    fn set_light_mode(&self, effect: Effect) -> Result<()> {
        self.set_light_mode(effect)
    }

    fn get_keys(&self, layer: KeyLayer) -> Result<Vec<u8>> {
        self.get_keys(layer)
    }

    fn set_keys(&self, layer: KeyLayer, keys: &[Key]) -> Result<()> {
        self.set_keys(layer, keys)
    }

    fn set_light_color(&self) -> Result<()> {
        self.set_light_color()
    }

    fn get_custom_light(&self) -> Result<Vec<u8>> {
        self.get_custom_light()
    }

    fn set_custom_light(&self, keys: &[Key]) -> Result<()> {
        self.set_custom_light(keys)
    }

    fn fetch_custom_light(&self, keys: &mut [Key]) -> Result<()> {
        self.fetch_custom_light(keys)
    }

    fn get_light_color(&self) -> Result<Vec<u8>> {
        self.get_light_color()
    }

    fn send_reset(&self) -> Result<()> {
        self.send_reset()
    }

    fn fetch_keys_layer(
        &self,
        layer: KeyLayer,
        macros: &[Macro],
        default_keys: &[Key],
    ) -> Result<Vec<Key>> {
        self.fetch_keys_layer(layer, macros, default_keys)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ============================================================================
// Implementation
// ============================================================================

impl AulaF75 {
    pub fn new() -> Result<Self> {
        let api = HidApi::new().context("Failed to initialize HID API")?;

        let device_info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VENDOR_ID
                    && d.product_id() == PRODUCT_ID
                    && d.interface_number() == 1
                    && d.usage_page() == 0xff00
            })
            .context("Vendor HID interface not found")?;

        log::info!(
            "Opening vendor HID interface at {}",
            device_info.path().to_string_lossy()
        );

        let device = device_info
            .open_device(&api)
            .context("Failed to open vendor HID interface")?;

        Ok(Self {
            device: Mutex::new(device),
        })
    }

    fn hid_send(&self, framed_packet: &[u8]) -> Result<()> {
        let device = self.device.lock().unwrap();
        log::trace!("Sending feature report: {} bytes", framed_packet.len());
        device
            .send_feature_report(framed_packet)
            .context("Failed to send feature report")?;
        Ok(())
    }

    fn hid_receive(&self) -> Result<Vec<u8>> {
        let device = self.device.lock().unwrap();
        let mut buffer = vec![REPORT_ID_RX; SEND_PAYLOAD_LENGTH + 1];

        let size = device
            .get_feature_report(&mut buffer)
            .context("Failed to receive feature report")?;

        if size == 0 {
            bail!("No data received");
        }

        log::trace!("Received feature report: {} bytes", size);
        Ok(buffer[1..size].to_vec())
    }

    // Packet Builders
    fn frame_packet(payload: &[u8]) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(SEND_PAYLOAD_LENGTH + 1);
        buffer.push(REPORT_ID_TX);

        let mut full_payload = vec![0u8; SEND_PAYLOAD_LENGTH];
        let copy_len = payload.len().min(SEND_PAYLOAD_LENGTH);
        full_payload[..copy_len].copy_from_slice(&payload[..copy_len]);

        buffer.extend_from_slice(&full_payload);
        buffer
    }

    // API Methods

    pub fn get_uuid(&self) -> Result<u64> {
        let tx = Self::frame_packet(&CMD_GET_UUID);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        if rx.len() < 13 {
            bail!("Invalid UUID response length: {}", rx.len());
        }

        let uuid = (u64::from(rx[7]) << 40)
            | (u64::from(rx[8]) << 32)
            | (u64::from(rx[9]) << 24)
            | (u64::from(rx[10]) << 16)
            | (u64::from(rx[11]) << 8)
            | u64::from(rx[12]);

        Ok(uuid)
    }

    pub fn send_reset(&self) -> Result<()> {
        let tx = Self::frame_packet(&CMD_RESET);
        self.hid_send(&tx)?;
        std::thread::sleep(Duration::from_millis(RESET_DELAY_MS));
        Ok(())
    }

    pub fn fetch_battery(&self) -> Result<BatteryStatus> {
        let tx = Self::frame_packet(&CMD_GET_BATTERY);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        let data = rx
            .get(7..)
            .ok_or_else(|| anyhow!("Battery response too short"))?;
        if data.len() < 2 {
            bail!("Battery data section too short");
        }

        Ok(BatteryStatus {
            charging: data[1] != 0,
            level: data[0],
        })
    }

    pub fn get_basic_info(&self) -> Result<DeviceInfo> {
        let tx = Self::frame_packet(&CMD_GET_BASIC_INFO);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        let buf = rx
            .get(7..)
            .ok_or_else(|| anyhow!("Basic info response too short"))?;
        if buf.len() != PAYLOAD_LENGTH_BASIC_INFO {
            bail!(
                "Expected {} bytes, got {}",
                PAYLOAD_LENGTH_BASIC_INFO,
                buf.len()
            );
        }

        let light_mode = u16::from_be_bytes([buf[9], buf[10]])
            .try_into()
            .unwrap_or(Effect::Off); // Fallback or handle error

        let mut light_params = Vec::with_capacity(LIGHT_PARAMS_LENGTH);
        for idx in 0..LIGHT_PARAMS_LENGTH {
            let offset = LIGHT_OFFSET - 7 + idx * 2;
            light_params.push(LightParam::new(buf[offset], buf[offset + 1]));
        }

        Ok(DeviceInfo {
            mac_mode: buf[0] != 0,
            polling_level: buf[1],
            low_latency: buf[3] == 3,
            win_lock: buf[15] != 0,
            light_mode,
            sleep_level: buf[24],
            light_params,
        })
    }

    /// Payload the legacy `set_basic_info` sends. It rebuilds the settings block from a few parsed
    /// fields plus hardcoded bytes, so it can clobber settings it does not model (e.g. effect colours).
    pub fn legacy_basic_payload(info: &DeviceInfo) -> Vec<u8> {
        let mut payload = vec![0u8; SEND_PAYLOAD_LENGTH];
        payload[..CMD_SET_BASIC_INFO.len()].copy_from_slice(&CMD_SET_BASIC_INFO);

        let light_mode = info.light_mode as u16;
        payload[7] = if info.mac_mode { 1 } else { 0 };
        payload[8] = info.polling_level;
        payload[9] = 3;
        payload[10] = if info.low_latency { 3 } else { 1 };
        payload[11] = 0;
        payload[12] = 0;
        payload[13] = 4;
        payload[14] = 4;
        payload[15] = 7;
        payload[16] = (light_mode >> 8) as u8;
        payload[17] = (light_mode & 0xFF) as u8;
        payload[18] = 32;
        payload[19] = 1;
        payload[22] = if info.win_lock { 1 } else { 0 };
        payload[25] = 1;
        payload[27] = 4;
        payload[28] = 1;
        payload[29] = 0;
        payload[30] = 255;
        payload[31] = info.sleep_level;
        payload[35] = 1;
        payload[37] = 1;
        payload[38] = 1;

        payload[LIGHT_OFFSET - 2] = 255;
        payload[LIGHT_OFFSET - 1] = 255;

        let mut end_index = LIGHT_OFFSET;
        for (idx, lp) in info.light_params.iter().enumerate() {
            let base = LIGHT_OFFSET + idx * 2;
            payload[base] = lp.bright;
            payload[base + 1] = lp.speed;
            end_index = base + 2;

            if let Some(magic) = Self::speed_to_magic(lp.speed as u16) {
                for &magic_idx in &MAGIC_INDEXES {
                    if magic_idx < payload.len() {
                        payload[magic_idx] = magic;
                    }
                }
            }
        }

        payload[end_index] = 90;
        payload[end_index + 1] = 165;
        payload
    }

    pub fn set_basic_info(&self, info: &DeviceInfo) -> Result<()> {
        let tx = Self::frame_packet(&Self::legacy_basic_payload(info));
        self.hid_send(&tx)?;
        Ok(())
    }

    /// The raw 128-byte settings block exactly as the keyboard reports it.
    pub fn get_basic_raw(&self) -> Result<Vec<u8>> {
        let tx = Self::frame_packet(&CMD_GET_BASIC_INFO);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;
        let buf = rx
            .get(7..)
            .ok_or_else(|| anyhow!("Basic info response too short"))?;
        if buf.len() != PAYLOAD_LENGTH_BASIC_INFO {
            bail!(
                "Expected {} bytes, got {}",
                PAYLOAD_LENGTH_BASIC_INFO,
                buf.len()
            );
        }
        Ok(buf.to_vec())
    }

    pub fn set_basic_raw(&self, raw: &[u8]) -> Result<()> {
        if raw.len() != PAYLOAD_LENGTH_BASIC_INFO {
            bail!("Settings block must be {} bytes", PAYLOAD_LENGTH_BASIC_INFO);
        }
        let mut payload = vec![0u8; SEND_PAYLOAD_LENGTH];
        payload[..CMD_SET_BASIC_INFO.len()].copy_from_slice(&CMD_SET_BASIC_INFO);
        payload[7..7 + raw.len()].copy_from_slice(raw);
        let tx = Self::frame_packet(&payload);
        self.hid_send(&tx)?;
        Ok(())
    }

    /// Change only the lighting effect; every other setting is written back exactly as read.
    pub fn set_light_mode(&self, effect: Effect) -> Result<()> {
        let mut raw = self.get_basic_raw()?;
        let m = (effect as u16).to_be_bytes();
        raw[9] = m[0];
        raw[10] = m[1];
        self.set_basic_raw(&raw)
    }

    pub fn get_keys(&self, layer: KeyLayer) -> Result<Vec<u8>> {
        let mut cmd = CMD_GET_KEYS;
        cmd[1] = layer as u8;
        let tx = Self::frame_packet(&cmd);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        if rx.len() != PAYLOAD_LENGTH_KEYS - 1 {
            bail!("Unexpected get_keys response len");
        }
        Ok(rx[7..].to_vec())
    }

    pub fn set_keys(&self, layer: KeyLayer, keys: &[Key]) -> Result<()> {
        let mut payload = [0u8; SEND_PAYLOAD_LENGTH];
        payload[..CMD_SET_KEY.len()].copy_from_slice(&CMD_SET_KEY);
        payload[1] = layer as u8;

        let offset = CMD_GET_KEYS.len();

        // Read current keymap first so unchanged keys aren't zeroed out
        if let Ok(current_keys) = self.get_keys(layer) {
            let copy_len = current_keys.len().min(SEND_PAYLOAD_LENGTH - offset);
            payload[offset..offset + copy_len].copy_from_slice(&current_keys[..copy_len]);
        }

        for key in keys.iter().filter(|k| k.layer == layer) {
            let idx = key.pos + offset;
            if idx + 4 > payload.len() {
                continue;
            }

            match key.key_type {
                KeyType::Basic => {
                    let value = parse_hex(&key.value);
                    let bytes = value.to_be_bytes();
                    payload[idx..idx + 4].copy_from_slice(&bytes);
                }
                KeyType::Macro => {
                    // Macro keys not yet implemented
                }
            }
        }

        payload[SEND_PAYLOAD_LENGTH - 2] = 90;
        payload[SEND_PAYLOAD_LENGTH - 1] = 165;
        let tx = Self::frame_packet(&payload);
        self.hid_send(&tx)?;
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    pub fn get_custom_light(&self) -> Result<Vec<u8>> {
        let tx = Self::frame_packet(&CMD_GET_CUSTOM_LIGHT);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        let data = rx
            .get(7..)
            .ok_or_else(|| anyhow!("Custom light response too short"))?;
        Ok(data.to_vec())
    }

    pub fn set_custom_light(&self, keys: &[Key]) -> Result<()> {
        let mut payload = vec![0u8; SEND_PAYLOAD_LENGTH];
        payload[..CMD_SET_CUSTOM_LIGHT.len()].copy_from_slice(&CMD_SET_CUSTOM_LIGHT);
        let prefix_len = CMD_SET_CUSTOM_LIGHT.len();

        for key in keys.iter().filter(|k| k.layer == KeyLayer::Normal) {
            let base = key.light_pos;
            if base + 252 + prefix_len >= payload.len() {
                continue;
            }

            payload[base + prefix_len] = key.color.r;
            payload[base + 126 + prefix_len] = key.color.g;
            payload[base + 252 + prefix_len] = key.color.b;
        }

        let tx = Self::frame_packet(&payload);
        self.hid_send(&tx)?;
        Ok(())
    }

    pub fn get_light_color(&self) -> Result<Vec<u8>> {
        let tx = Self::frame_packet(&CMD_GET_LIGHT_COLOR);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;
        Ok(rx.to_vec())
    }

    pub fn set_light_color(&self) -> Result<()> {
        let mut payload = [0u8; SEND_PAYLOAD_LENGTH];
        payload[..CMD_SET_LIGHT_COLOR.len()].copy_from_slice(&CMD_SET_LIGHT_COLOR);

        let offset = CMD_SET_LIGHT_COLOR.len();
        payload[offset..offset + SL.len()].copy_from_slice(&SL);

        let offsets = [28, 49, 91, 112, 154, 175, 217, 238, 259, 280];
        for offset in offsets {
            if offset + 2 < payload.len() {
                payload[offset] = 0;
                payload[offset + 1] = 255;
                payload[offset + 2] = 0;
            }
        }

        let tx = Self::frame_packet(&payload);
        self.hid_send(&tx)?;
        Ok(())
    }

    pub fn fetch_keys_layer(
        &self,
        layer: KeyLayer,
        _macros: &[Macro],
        default_keys: &[Key],
    ) -> Result<Vec<Key>> {
        let key_data = self.get_keys(layer)?;
        let lights = self.get_custom_light()?;
        let lookup = build_key_lookup(default_keys);

        let mut keys = Vec::new();

        for i in (0..key_data.len()).step_by(4) {
            if i + 3 >= key_data.len() {
                break;
            }

            let light_pos = i / 4;
            let value = u32::from_be_bytes([
                key_data[i],
                key_data[i + 1],
                key_data[i + 2],
                key_data[i + 3],
            ]);

            if value == 0 {
                continue;
            }

            if key_data[i + 3] == 3 {
                log::debug!("Skipping macro key at position {}", i);
                continue;
            }

            let value_str = format!("0x{:08x}", value);
            let name = lookup
                .get(&value_str)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let color = extract_color_from_lights(&lights, light_pos);

            keys.push(Key {
                name,
                value: value_str,
                pos: i,
                light_pos,
                effect_pos: 0, // todo
                layer,
                key_type: KeyType::Basic,
                color,
                ..Default::default()
            });
        }

        Ok(keys)
    }

    pub fn fetch_custom_light(&self, keys: &mut [Key]) -> Result<()> {
        let lights = self.get_custom_light()?;

        for key in keys.iter_mut().filter(|k| k.layer == KeyLayer::Normal) {
            key.color = extract_color_from_lights(&lights, key.light_pos);
        }

        Ok(())
    }

    fn speed_to_magic(speed: u16) -> Option<u8> {
        match speed {
            256 => Some(16),
            263 => Some(23),
            512 => Some(32),
            519 => Some(39),
            768 => Some(48),
            775 => Some(55),
            1024 => Some(64),
            1031 => Some(71),
            _ => None,
        }
    }
}
