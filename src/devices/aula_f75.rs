use crate::config::{KEYMAP_BYTES, LIGHT_BYTES, validate_keys};
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
// Custom RGB uses three 126-byte planes; 0x8a/0x0a is the effect palette.
const CMD_GET_CUSTOM_LIGHT: [u8; 7] = [134, 0, 0, 1, 0, 128, 1];
const CMD_SET_CUSTOM_LIGHT: [u8; 7] = [6, 0, 0, 1, 0, 126, 1];
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

        let light_mode = u16::from(buf[10]).try_into().unwrap_or(Effect::Off); // Fallback or handle error

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

    /// DANGEROUS: rebuilds the whole settings block from a few fields plus hardcoded bytes and can
    /// corrupt it (this broke Fn+combos, see docs/fn-layer-fix.md). Use `set_light_mode` or
    /// `get_basic_raw` + `set_basic_raw` (read-modify-write) instead.
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
        thread::sleep(Duration::from_millis(20));
        Ok(())
    }

    /// Change the effect and its custom-table enable flag, preserving other settings.
    pub fn set_light_mode(&self, effect: Effect) -> Result<()> {
        let mut raw = self.get_basic_raw()?;
        patch_light_mode(&mut raw, effect);
        self.set_basic_raw(&raw)
    }

    pub fn get_keys(&self, layer: KeyLayer) -> Result<Vec<u8>> {
        self.get_keys_raw_layer(layer as u8)
    }

    /// Read a keymap layer by raw id (the keyboard may have more layers than `KeyLayer` names).
    pub fn get_keys_raw_layer(&self, layer: u8) -> Result<Vec<u8>> {
        let mut cmd = CMD_GET_KEYS;
        cmd[1] = layer;
        let tx = Self::frame_packet(&cmd);
        self.hid_send(&tx)?;
        let rx = self.hid_receive()?;

        if rx.len() != PAYLOAD_LENGTH_KEYS - 1 {
            bail!("Unexpected get_keys response len");
        }
        Ok(rx[7..].to_vec())
    }

    pub fn set_keys(&self, layer: KeyLayer, keys: &[Key]) -> Result<()> {
        set_keys_with(
            layer,
            keys,
            || self.get_keys(layer),
            |payload| {
                self.hid_send(&Self::frame_packet(payload))?;
                thread::sleep(Duration::from_millis(100));
                Ok(())
            },
        )
    }

    pub fn get_custom_light(&self) -> Result<Vec<u8>> {
        let tx = Self::frame_packet(&CMD_GET_CUSTOM_LIGHT);
        self.hid_send(&tx)?;
        thread::sleep(Duration::from_millis(20));
        let rx = self.hid_receive()?;
        anyhow::ensure!(
            rx.get(..7) == Some(CMD_GET_CUSTOM_LIGHT.as_slice()),
            "Unexpected RGB response header"
        );
        let data = rx
            .get(7..)
            .ok_or_else(|| anyhow!("RGB response too short"))?;
        anyhow::ensure!(
            data.len() == LIGHT_BYTES,
            "Expected {LIGHT_BYTES} RGB bytes, got {}",
            data.len()
        );
        Ok(data.to_vec())
    }

    pub fn set_custom_light(&self, keys: &[Key]) -> Result<()> {
        set_custom_light_with(
            keys,
            || self.get_custom_light(),
            |payload| {
                // Read both blocks before mutation; enable the table before writing colours.
                let original = self.get_basic_raw()?;
                let mut settings = original.clone();
                patch_light_mode(&mut settings, Effect::Custom);
                self.set_basic_raw(&settings)?;
                let result = (|| -> Result<()> {
                    self.hid_send(&Self::frame_packet(payload))?;
                    thread::sleep(Duration::from_millis(20));
                    let actual = self.get_custom_light()?;
                    anyhow::ensure!(
                        actual == payload[7..7 + LIGHT_BYTES],
                        "RGB write did not read back correctly"
                    );
                    anyhow::ensure!(
                        self.get_basic_raw()? == settings,
                        "Custom settings did not read back correctly"
                    );
                    Ok(())
                })();
                if result.is_err() {
                    self.set_basic_raw(&original)
                        .context("Failed to restore lighting settings after RGB error")?;
                }
                result?;
                Ok(())
            },
        )
    }

    pub fn get_light_color(&self) -> Result<Vec<u8>> {
        let command = [138, 0, 0, 1, 0, 128, 2];
        self.hid_send(&Self::frame_packet(&command))?;
        thread::sleep(Duration::from_millis(20));
        let rx = self.hid_receive()?;
        Ok(rx
            .get(7..)
            .ok_or_else(|| anyhow!("Colour profile response too short"))?
            .to_vec())
    }

    /// The legacy no-argument initializer overwrote the RGB table with hardcoded data.
    pub fn set_light_color(&self) -> Result<()> {
        bail!("Use set_custom_light with explicit key colours instead of the legacy initializer")
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

fn patch_light_mode(raw: &mut [u8], effect: Effect) {
    // Byte 9 is a separate flag, not the high byte of a u16 mode.
    raw[9] = u8::from(effect == Effect::Custom);
    raw[10] = effect as u8;
}

/// Read/patch/write helpers shared by the HID entry points and transport-free tests.
fn set_keys_with(
    layer: KeyLayer,
    keys: &[Key],
    read: impl FnOnce() -> Result<Vec<u8>>,
    write: impl FnOnce(&[u8]) -> Result<()>,
) -> Result<()> {
    validate_keys(keys)?;
    if !keys.iter().any(|k| k.layer == layer) {
        return Ok(());
    }
    let current = read().context("Cannot read current keymap; refusing to write")?;
    anyhow::ensure!(
        current.len() == KEYMAP_BYTES,
        "Invalid current keymap length: expected {KEYMAP_BYTES}, got {}; refusing to write",
        current.len()
    );
    let mut payload = [0u8; SEND_PAYLOAD_LENGTH];
    payload[..CMD_SET_KEY.len()].copy_from_slice(&CMD_SET_KEY);
    payload[1] = layer as u8;
    let offset = CMD_SET_KEY.len();
    payload[offset..offset + KEYMAP_BYTES].copy_from_slice(&current);
    for key in keys.iter().filter(|k| k.layer == layer) {
        let idx = offset + key.pos;
        payload[idx..idx + 4].copy_from_slice(&parse_hex(&key.value)?.to_be_bytes());
    }
    payload[SEND_PAYLOAD_LENGTH - 2..].copy_from_slice(&[90, 165]);
    write(&payload)
}

fn set_custom_light_with(
    keys: &[Key],
    read: impl FnOnce() -> Result<Vec<u8>>,
    write: impl FnOnce(&[u8]) -> Result<()>,
) -> Result<()> {
    validate_keys(keys)?;
    if !keys.iter().any(|k| k.layer == KeyLayer::Normal) {
        return Ok(());
    }
    let current = read().context("Cannot read current custom lighting; refusing to write")?;
    anyhow::ensure!(
        current.len() == LIGHT_BYTES,
        "Invalid current lighting length: expected {LIGHT_BYTES}, got {}; refusing to write",
        current.len()
    );
    let mut payload = [0u8; SEND_PAYLOAD_LENGTH];
    let offset = CMD_SET_CUSTOM_LIGHT.len();
    payload[..offset].copy_from_slice(&CMD_SET_CUSTOM_LIGHT);
    payload[offset..offset + LIGHT_BYTES].copy_from_slice(&current);
    for key in keys.iter().filter(|k| k.layer == KeyLayer::Normal) {
        payload[offset + key.light_pos] = key.color.r;
        payload[offset + key.light_pos + 126] = key.color.g;
        payload[offset + key.light_pos + 252] = key.color.b;
    }
    write(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    fn key(layer: KeyLayer) -> Key {
        Key {
            light_pos: 125,
            color: Color::create(10, 20, 30),
            ..Key::new_basic("Test".into(), "0x12345678".into(), 500, layer)
        }
    }

    #[test]
    fn custom_mode_enables_table_and_preserves_other_settings() {
        let original = vec![37; 128];
        let mut raw = original.clone();
        patch_light_mode(&mut raw, Effect::Custom);
        assert_eq!(&raw[9..11], &[1, 21]);
        assert_eq!(&raw[..9], &original[..9]);
        assert_eq!(&raw[11..], &original[11..]);
        patch_light_mode(&mut raw, Effect::Reaction);
        assert_eq!(&raw[9..11], &[0, 12]);
    }

    #[test]
    fn failed_or_malformed_reads_never_write() {
        for current in [
            Err(anyhow!("read failed")),
            Ok(vec![1; KEYMAP_BYTES - 1]),
            Ok(vec![1; KEYMAP_BYTES + 1]),
        ] {
            let result = set_keys_with(
                KeyLayer::Normal,
                &[key(KeyLayer::Normal)],
                || current,
                |_| panic!("must not write after failed/invalid read"),
            );
            assert!(result.is_err());
        }
        for current in [
            Err(anyhow!("read failed")),
            Ok(vec![1; LIGHT_BYTES - 1]),
            Ok(vec![1; LIGHT_BYTES + 1]),
        ] {
            let result = set_custom_light_with(
                &[key(KeyLayer::Normal)],
                || current,
                |_| panic!("must not write after failed/invalid read"),
            );
            assert!(result.is_err());
        }
    }

    #[test]
    fn validation_precedes_all_io_even_for_other_layers() {
        let mut invalid = vec![
            Key {
                value: "bad hex".into(),
                ..key(KeyLayer::Fn)
            },
            Key {
                pos: usize::MAX,
                ..key(KeyLayer::Fn)
            },
            Key {
                light_pos: 126,
                ..key(KeyLayer::Fn)
            },
            Key {
                key_type: KeyType::Macro,
                ..key(KeyLayer::Fn)
            },
        ];
        invalid.push(Key {
            pos: 501,
            ..key(KeyLayer::Fn)
        });
        for k in invalid {
            let keys = [key(KeyLayer::Normal), k];
            assert!(
                set_keys_with(
                    KeyLayer::Normal,
                    &keys,
                    || panic!("must validate before reading"),
                    |_| panic!("must not write")
                )
                .is_err()
            );
            assert!(
                set_custom_light_with(
                    &keys,
                    || panic!("must validate before reading"),
                    |_| panic!("must not write")
                )
                .is_err()
            );
        }
        let keys = [key(KeyLayer::Normal), key(KeyLayer::Normal)];
        assert!(
            set_keys_with(
                KeyLayer::Normal,
                &keys,
                || panic!("must validate duplicates before reading"),
                |_| panic!("must not write")
            )
            .is_err()
        );
    }

    #[test]
    fn partial_maps_preserve_every_untouched_byte_in_all_layers() {
        for layer in [KeyLayer::Normal, KeyLayer::Fn, KeyLayer::Fn1] {
            let original: Vec<u8> = (0..KEYMAP_BYTES).map(|i| (i % 251) as u8).collect();
            let mut first = key(layer);
            first.pos = 0;
            first.light_pos = 0;
            first.value = "0x0".into();
            let mut expected = original.clone();
            expected[..4].copy_from_slice(&[0; 4]);
            expected[500..].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
            let mut writes = 0;
            set_keys_with(
                layer,
                &[first, key(layer)],
                || Ok(original),
                |payload| {
                    assert_eq!(payload[1], layer as u8);
                    assert_eq!(&payload[7..7 + KEYMAP_BYTES], expected);
                    assert_eq!(&payload[517..], &[90, 165]);
                    writes += 1;
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(writes, 1);
        }
    }

    #[test]
    fn partial_lighting_targets_rgb_command_and_preserves_other_slots() {
        let original: Vec<u8> = (0..LIGHT_BYTES).map(|i| (i % 251) as u8).collect();
        let mut expected = original.clone();
        expected[125] = 10;
        expected[251] = 20;
        expected[377] = 30;
        let mut writes = 0;
        // Other layers can reuse the slot, but must not change custom lighting.
        set_custom_light_with(
            &[
                key(KeyLayer::Normal),
                Key {
                    color: Color::default(),
                    ..key(KeyLayer::Fn)
                },
            ],
            || Ok(original),
            |payload| {
                assert_eq!(&payload[..7], &[0x06, 0, 0, 1, 0, 126, 0x01]);
                assert_eq!(&payload[7..7 + LIGHT_BYTES], expected);
                writes += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(writes, 1);
    }

    #[test]
    fn empty_or_unselected_layers_do_not_write() {
        set_keys_with(
            KeyLayer::Fn,
            &[key(KeyLayer::Normal)],
            || panic!("nothing to read"),
            |_| panic!("nothing to write"),
        )
        .unwrap();
        set_custom_light_with(
            &[key(KeyLayer::Fn)],
            || panic!("nothing to read"),
            |_| panic!("nothing to write"),
        )
        .unwrap();
        set_keys_with(
            KeyLayer::Normal,
            &[],
            || panic!("nothing to read"),
            |_| panic!("nothing to write"),
        )
        .unwrap();
    }
}
