//! GPUI configurator: click a key, pick its function and colour, then apply to the keyboard.
use aula_f75::types::{Color, Effect, EffectColor, Key, KeyLayer};
use aula_f75::{
    DEFAULT_CONFIG, connect, parse_config, parse_profile, serialize_profile_with_color,
    validate_keys,
};
use gpui::{
    App, Application, BoxShadow, Context, FocusHandle, Hsla, KeyDownEvent, Window, WindowOptions,
    div, point, prelude::*, px, rgb,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Key unit in px. Physical layout below is keyed by `light_pos` (matrix index).
const U: f32 = 54.0;
const FUNCTION_ROW_GAP: f32 = 0.3;

#[path = "gui/color_picker.rs"]
mod color_picker;
use color_picker::ColorPicker;

const FINNISH_ANSI: &str = include_str!("../../examples/finnish-ansi.toml");

/// Key-function presets: only the key codes change, per-key colours are kept.
const LAYOUTS: [(&str, Option<&str>); 2] = [("Stock", None), ("Finnish ANSI", Some(FINNISH_ANSI))];

enum Cell {
    Gap(f32),
    K(usize, f32),
}

/// F75 physical layout: rows of (light_pos, width in units) with gaps.
// Function-row group gaps follow the top-down F75 product photo:
// https://aulagear.com/cdn/shop/files/203A5822.jpg
fn layout() -> Vec<Vec<Cell>> {
    use Cell::{Gap, K};
    let ks = |ps: &[usize], w: f32| -> Vec<Cell> { ps.iter().map(|&p| K(p, w)).collect() };
    let cat = |parts: Vec<Vec<Cell>>| -> Vec<Cell> { parts.into_iter().flatten().collect() };
    vec![
        cat(vec![
            ks(&[0], 1.0),
            vec![Gap(1.0)],
            ks(&[12, 18, 24, 30], 1.0),
            vec![Gap(0.25)],
            ks(&[36, 42, 48, 54], 1.0),
            vec![Gap(0.25)],
            ks(&[60, 66, 72, 78], 1.0),
            vec![Gap(0.5)],
            ks(&[84], 1.0),
        ]),
        cat(vec![
            ks(&[1, 7, 13, 19, 25, 31, 37, 43, 49, 55, 61, 67, 73], 1.0),
            ks(&[79], 2.0),
            ks(&[85], 1.0),
        ]),
        cat(vec![
            ks(&[2], 1.5),
            ks(&[8, 14, 20, 26, 32, 38, 44, 50, 56, 62, 68, 74], 1.0),
            ks(&[80], 1.5),
            ks(&[86], 1.0),
        ]),
        cat(vec![
            ks(&[3], 1.75),
            ks(&[9, 15, 21, 27, 33, 39, 45, 51, 57, 63, 69], 1.0),
            ks(&[81], 2.25),
            ks(&[87], 1.0),
        ]),
        cat(vec![
            ks(&[4], 2.25),
            ks(&[10, 16, 22, 28, 34, 40, 46, 52, 58, 64], 1.0),
            ks(&[70], 1.75),
            ks(&[82, 88], 1.0),
        ]),
        cat(vec![
            ks(&[5, 11, 17], 1.25),
            ks(&[35], 6.25),
            ks(&[53, 59], 1.25),
            vec![Gap(0.5)],
            ks(&[77, 83, 89], 1.0),
        ]),
    ]
}

/// (value, label) choices for the key-function list.
fn choices() -> Vec<(String, String)> {
    let mut v: Vec<(u32, String)> = Vec::new();
    for (i, c) in ('A'..='Z').enumerate() {
        v.push((0x04 + i as u32, c.to_string()));
    }
    for (i, c) in "1234567890".chars().enumerate() {
        v.push((0x1e + i as u32, c.to_string()));
    }
    for (code, name) in [
        (0x28, "Enter"),
        (0x29, "Esc"),
        (0x2a, "Bksp"),
        (0x2b, "Tab"),
        (0x2c, "Space"),
        (0x2d, "-"),
        (0x2e, "="),
        (0x2f, "["),
        (0x30, "]"),
        (0x31, "\\"),
        (0x33, ";"),
        (0x34, "'"),
        (0x35, "`"),
        (0x36, ","),
        (0x37, "."),
        (0x38, "/"),
        (0x39, "Caps"),
        (0x46, "PrtSc"),
        (0x47, "ScrLk"),
        (0x48, "Pause"),
        (0x49, "Insert"),
        (0x4a, "Home"),
        (0x4b, "PgUp"),
        (0x4c, "Delete"),
        (0x4d, "End"),
        (0x4e, "PgDn"),
        (0x4f, "Right"),
        (0x50, "Left"),
        (0x51, "Down"),
        (0x52, "Up"),
    ] {
        v.push((code, name.to_string()));
    }
    for i in 0..12u32 {
        v.push((0x3a + i, format!("F{}", i + 1)));
    }
    let mut out: Vec<(String, String)> = v
        .into_iter()
        .map(|(c, n)| (format!("0x{c:08x}"), n))
        .collect();
    for (code, name) in [
        (0x00010000u32, "L Ctrl"),
        (0x00020000, "L Shift"),
        (0x00040000, "L Alt"),
        (0x00080000, "L Win"),
        (0x00100000, "R Ctrl"),
        (0x00200000, "R Shift"),
        (0x00400000, "R Alt"),
        (0x0d000000, "Fn"),
    ] {
        out.push((format!("0x{code:08x}"), name.to_string()));
    }
    out
}

const PALETTE: [u32; 10] = [
    0xff0000, 0xff7f00, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0x8b00ff, 0xff00ff, 0xffffff,
    0x000000,
];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Keys,
    Lighting,
}

/// Picker entries, indexed by firmware mode id (see `Effect`).
const EFFECTS: [(Effect, &str); 21] = [
    (Effect::Off, "Off"),
    (Effect::Mode1, "Mode 1"),
    (Effect::Respire, "Respire"),
    (Effect::Rainbow, "Rainbow"),
    (Effect::FlashAway, "Flash away"),
    (Effect::Raindrops, "Raindrops"),
    (Effect::GradientDrift, "Gradient drift"),
    (Effect::RipplesShining, "Ripples shining"),
    (Effect::StarsTwinkle, "Stars twinkle"),
    (Effect::Mode9, "Mode 9"),
    (Effect::RetroSnake, "Retro snake"),
    (Effect::SineWave, "Sine wave"),
    (Effect::Reaction, "Reaction"),
    (Effect::Blossoming, "Blossoming"),
    (Effect::Mode14, "Mode 14"),
    (Effect::ColorfulWaterfall, "Colorful waterfall"),
    (Effect::Mode16, "Mode 16"),
    (Effect::CenterBurst, "Center burst"),
    (Effect::Mode18, "Mode 18"),
    (Effect::Mode19, "Mode 19"),
    (Effect::Custom, "Custom per-key"),
];

/// Indices into `EFFECTS`, grouped for the picker.
const EFFECT_GROUPS: [(&str, &[usize]); 5] = [
    ("Basic", &[0, 20]),
    ("Flowing", &[2, 3, 6, 11, 13, 15, 17]),
    ("Animated", &[5, 8, 10]),
    ("Reactive (press keys)", &[4, 7, 12]),
    (
        "Unverified (showed no light in testing)",
        &[1, 9, 14, 16, 18, 19],
    ),
];

fn hsv(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 60.0;
    let (i, f) = (h.floor() as i32, h - h.floor());
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Blend two 0xRRGGBB colours (`t` = share of `b`).
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xff) as f32, ((b >> s) & 0xff) as f32);
        ((x + (y - x) * t) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

fn hash(a: i32, b: i32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x9e3779b1) ^ (b as u32).wrapping_mul(0x85ebca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

/// Approximate on-screen preview of a firmware effect (not the firmware's exact animation).
/// `x`,`y` are the key's position in layout units, `t` is seconds, `base` the key's own colour.
/// Reactive effects animate from a simulated key press that moves around the board.
fn preview(effect: Effect, x: f32, y: f32, t: f32, base: (u8, u8, u8)) -> (u8, u8, u8) {
    let scale = |c: (u8, u8, u8), k: f32| {
        let k = k.clamp(0.0, 1.0);
        (
            (c.0 as f32 * k) as u8,
            (c.1 as f32 * k) as u8,
            (c.2 as f32 * k) as u8,
        )
    };
    let (cx, cy) = (7.5, 2.5);
    let dist = ((x - cx).powi(2) + ((y - cy) * 1.4).powi(2)).sqrt();
    // simulated key press: a new one every 1.8s at a pseudo-random key
    let epoch = (t / 1.8).floor();
    let age = t - epoch * 1.8;
    let (px, py) = (
        (hash(epoch as i32, 1) * 13.0).floor() + 0.5,
        (hash(epoch as i32, 2) * 5.0).floor(),
    );
    let press_hue = hash(epoch as i32, 3) * 360.0;
    match effect {
        Effect::Off
        | Effect::Mode1
        | Effect::Mode9
        | Effect::Mode14
        | Effect::Mode16
        | Effect::Mode18
        | Effect::Mode19 => (0, 0, 0),
        Effect::Custom => base,
        Effect::Respire => scale(hsv(210.0, 0.8, 1.0), 0.5 + 0.5 * (t * 2.0).sin()),
        // whole board one colour, cycling
        Effect::Rainbow => hsv(t * 50.0, 1.0, 1.0),
        Effect::SineWave => hsv(t * 90.0, 1.0, 1.0),
        // slow gradient drifting across the board
        Effect::GradientDrift => hsv(x * 9.0 + t * 25.0, 1.0, 1.0),
        // light beams out along the pressed key's row, both ways
        Effect::FlashAway => {
            const PERIOD: f32 = 0.35;
            let now = (t / PERIOD).floor();
            let mut best = (0.0f32, 0.0f32); // (brightness, hue)
            for back in 0..3 {
                let e = (now - back as f32) as i32;
                let (kx, ky) = (
                    (hash(e, 21) * 13.0).floor() + 0.5,
                    (hash(e, 22) * 5.0).floor(),
                );
                if (y - ky).abs() < 0.5 {
                    let a = t - e as f32 * PERIOD;
                    let (d, front) = ((x - kx).abs(), a * 22.0);
                    let k = if d <= front {
                        1.0 - (front - d) / 4.0
                    } else {
                        0.0
                    };
                    let k = k.clamp(0.0, 1.0) * (1.0 - a / 1.0).max(0.0);
                    if k > best.0 {
                        best = (k, hash(e, 23) * 360.0);
                    }
                }
            }
            scale(hsv(best.1, 1.0, 1.0), best.0)
        }
        // circular ring expanding from the pressed key
        Effect::RipplesShining => {
            let d = ((x - px).powi(2) + ((y - py) * 1.1).powi(2)).sqrt();
            let k = 1.0 - (d - age * 9.0).abs() * 0.55;
            scale(hsv(press_hue + d * 18.0, 0.9, 1.0), k * (1.2 - age / 1.8))
        }
        // only the pressed key lights, in its own colour
        Effect::Reaction => {
            // quick typing: a new press every 0.3s, the last few still fading
            const PERIOD: f32 = 0.3;
            let now = (t / PERIOD).floor();
            let mut k = 0.0f32;
            for back in 0..4 {
                let e = (now - back as f32) as i32;
                let (kx, ky) = (
                    (hash(e, 11) * 13.0).floor() + 0.5,
                    (hash(e, 12) * 5.0).floor(),
                );
                if (x - kx).abs() < 0.6 && (y - ky).abs() < 0.5 {
                    let a = t - e as f32 * PERIOD;
                    k = k.max(1.0 - a / 0.9);
                }
            }
            scale(base, k)
        }
        Effect::Raindrops => {
            let tt = t * 3.0 + y * 0.7;
            let h = hash(x as i32, tt.floor() as i32);
            scale(
                hsv(200.0, 0.7, 1.0),
                if h > 0.8 { 1.0 - tt.fract() } else { 0.05 },
            )
        }
        // random keys light up in random colours
        Effect::StarsTwinkle => {
            let slot = (t * 2.0 + hash(x as i32, y as i32) * 4.0) as i32;
            let h = hash(x as i32 * 7 + y as i32, slot);
            scale(
                hsv(h * 360.0 * 3.0, 1.0, 1.0),
                if h > 0.6 { 1.0 } else { 0.0 },
            )
        }
        Effect::RetroSnake => {
            let idx = (y as i32 * 16
                + if y as i32 % 2 == 0 {
                    x as i32
                } else {
                    15 - x as i32
                }) as f32;
            let head = (t * 14.0) % 96.0;
            let behind = (head - idx).rem_euclid(96.0);
            scale(
                hsv(120.0, 1.0, 1.0),
                if behind < 10.0 {
                    1.0 - behind / 10.0
                } else {
                    0.03
                },
            )
        }
        Effect::ColorfulWaterfall => hsv(y * 55.0 - t * 110.0 + x * 6.0, 1.0, 1.0),
        Effect::Blossoming => hsv(
            dist * 30.0 - t * 80.0,
            0.9,
            0.5 + 0.5 * (dist * 0.9 - t * 4.0).sin(),
        ),
        // solid gradient radiating from the centre
        Effect::CenterBurst => hsv(dist * 30.0 - t * 90.0, 1.0, 1.0),
    }
}

struct Configurator {
    keys: Vec<Key>,
    selected: Option<usize>,
    path: PathBuf,
    battery: Option<(u8, bool)>,
    status: String,
    choices: Vec<(String, String)>,
    tab: Tab,
    effect: Effect,
    started: Instant,
    effect_dirty: bool,
    effect_color: EffectColor,
    effect_color_dirty: bool,
    custom: Vec<u32>,
    picker: ColorPicker,
    orig_keys: Vec<Key>,
    connected: bool,
    layout: Option<usize>,
    profiles: Vec<String>,
    active_profile: Option<String>,
    /// Name being typed for a new profile (None = not naming).
    naming: Option<String>,
    name_focus: FocusHandle,
}

/// Where named profiles live, next to the default config file.
fn profiles_dir(config: &std::path::Path) -> PathBuf {
    config
        .parent()
        .map(|p| p.join("profiles"))
        .unwrap_or_else(|| PathBuf::from("profiles"))
}

fn list_profiles(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension()? == "toml").then(|| p.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    v.sort();
    v
}

impl Configurator {
    fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let (keys, status) = match std::fs::read_to_string(&path).map(|s| parse_config(&s)) {
            Ok(Ok(k)) => (k, format!("Loaded {}", path.display())),
            Ok(Err(e)) => (
                parse_config(DEFAULT_CONFIG).expect("bundled default config"),
                format!(
                    "Invalid config ({}): {e:#}; showing bundled defaults",
                    path.display()
                ),
            ),
            Err(e) => (
                parse_config(DEFAULT_CONFIG).expect("bundled default config"),
                format!(
                    "Could not read {}: {e}; showing bundled defaults",
                    path.display()
                ),
            ),
        };
        let mut s = Self {
            connected: false,
            layout: None,
            profiles: list_profiles(&profiles_dir(&path)),
            active_profile: None,
            naming: None,
            name_focus: cx.focus_handle(),
            keys,
            selected: None,
            path,
            status,
            battery: None,
            choices: choices(),
            tab: if std::env::var_os("AULA_TAB_LIGHTING").is_some() {
                Tab::Lighting
            } else {
                Tab::Keys
            },
            effect: Effect::Off,
            started: Instant::now(),
            effect_dirty: false,
            effect_color: EffectColor {
                color: Color::create(255, 255, 255),
                rainbow: true,
            },
            effect_color_dirty: false,
            custom: Vec::new(),
            picker: ColorPicker::default(),
            orig_keys: Vec::new(),
        };
        s.orig_keys = s.keys.clone();
        s.refresh_device();
        if s.connected {
            // The keyboard is the source of truth: show what it is actually doing.
            let fallback = std::mem::take(&mut s.status);
            s.read_from_keyboard();
            if s.status.starts_with("Read failed") {
                s.status = fallback;
            } else {
                s.status = "Loaded current keymap + colours from keyboard".into();
            }
        }
        s
    }

    fn label_for(&self, value: &str) -> Option<&str> {
        self.choices
            .iter()
            .find(|(v, _)| v == value)
            .map(|(_, n)| n.as_str())
    }

    fn editing_effect_color(&self) -> bool {
        self.tab == Tab::Lighting && self.effect.supports_color()
    }

    fn edited_color(&self) -> Option<(u8, u8, u8)> {
        let c = if self.editing_effect_color() {
            &self.effect_color.color
        } else if self.tab == Tab::Lighting && self.effect != Effect::Custom {
            return None;
        } else {
            &self.keys.get(self.selected?)?.color
        };
        Some((c.r, c.g, c.b))
    }

    fn sync_picker(&mut self) {
        if let Some((r, g, b)) = self.edited_color() {
            self.picker.sync_rgb(r, g, b);
        }
    }

    fn set_edited_color(&mut self, color: Color) {
        if self.editing_effect_color() {
            self.effect_color.color = color;
            self.effect_color.rainbow = false;
            self.effect_color_dirty = true;
            self.effect_dirty = true;
            self.status =
                "Effect colour edited: Apply uses this colour for all reacting keys".into();
        } else if let Some(i) = self.selected {
            if self.tab == Tab::Lighting && self.effect != Effect::Custom {
                return;
            }
            self.keys[i].color = color;
            self.effect = Effect::Custom;
            self.effect_dirty = true;
            self.status = "Colour edited: Apply enables Custom per-key lighting".into();
        }
    }

    fn update_picker_color(&mut self) {
        let (r, g, b) = self.picker.rgb();
        self.set_edited_color(Color::create(r, g, b));
    }

    fn set_color(&mut self, rgb_val: u32) {
        self.set_edited_color(Color::create(
            (rgb_val >> 16) as u8,
            (rgb_val >> 8) as u8,
            rgb_val as u8,
        ));
        self.sync_picker();
    }

    fn set_channel(&mut self, ch: usize, v: u8) {
        if let Some((r, g, b)) = self.edited_color() {
            let (r, g, b) = match ch {
                0 => (v, g, b),
                1 => (r, v, b),
                _ => (r, g, v),
            };
            self.set_edited_color(Color::create(r, g, b));
            self.sync_picker();
        }
    }

    fn choose_effect(&mut self, effect: Effect) {
        self.effect = effect;
        if effect.supports_color() && !self.effect_color_dirty {
            match connect().and_then(|d| d.get_effect_color(effect)) {
                Ok(color) => self.effect_color = color,
                Err(e) => self.status = format!("Could not read effect colour: {e:#}"),
            }
        }
        self.effect_dirty = true;
        self.sync_picker();
    }

    /// Re-read battery and (unless you have an unapplied choice) the current lighting effect.
    fn refresh_device(&mut self) {
        let Ok(d) = connect() else {
            self.battery = None;
            self.connected = false;
            return;
        };
        self.connected = true;
        self.battery = d.fetch_battery().ok().map(|b| (b.level, b.charging));
        if !self.effect_dirty {
            if let Ok(info) = d.get_basic_info() {
                self.effect = info.light_mode;
                if self.effect.supports_color() {
                    if let Ok(color) = d.get_effect_color(self.effect) {
                        self.effect_color = color;
                    }
                }
                self.sync_picker();
            }
        }
    }

    /// File that Save/Reload act on: the active profile, else the default config.
    fn current_path(&self) -> PathBuf {
        match &self.active_profile {
            Some(n) => profiles_dir(&self.path).join(format!("{n}.toml")),
            None => self.path.clone(),
        }
    }

    fn write_to(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let color = self
            .effect
            .supports_color()
            .then(|| self.effect_color.clone());
        let s = serialize_profile_with_color(&self.keys, Some(self.effect as u16), color)?;
        Ok(std::fs::write(path, s)?)
    }

    fn save(&mut self) {
        let path = self.current_path();
        self.status = match self.write_to(&path) {
            Ok(()) => format!("Saved {}", path.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    /// Discard unsaved edits: re-read the current file from disk and re-check the keyboard.
    fn reload(&mut self) {
        let path = self.current_path();
        self.refresh_profiles();
        match std::fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|s| parse_profile(&s))
        {
            Ok(p) => {
                self.keys = p.keys;
                if let Some(e) = p.effect.and_then(|e| Effect::try_from(e).ok()) {
                    self.effect = e;
                    self.effect_dirty = true;
                }
                self.effect_color_dirty = p.effect_color.is_some();
                if let Some(color) = p.effect_color {
                    self.effect_color = color;
                }
                self.selected = None;
                self.sync_picker();
                self.refresh_device();
                self.status = format!("Reloaded {}", path.display());
            }
            Err(e) => {
                self.refresh_device();
                self.status = format!("Reload failed ({}): {e:#}", path.display());
            }
        }
    }

    fn refresh_profiles(&mut self) {
        self.profiles = list_profiles(&profiles_dir(&self.path));
    }

    fn load_profile(&mut self, name: &str) {
        let path = profiles_dir(&self.path).join(format!("{name}.toml"));
        match std::fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|s| parse_profile(&s))
        {
            Ok(p) => {
                self.keys = p.keys;
                if let Some(e) = p.effect.and_then(|e| Effect::try_from(e).ok()) {
                    self.effect = e;
                    self.effect_dirty = true;
                }
                self.effect_color_dirty = p.effect_color.is_some();
                if let Some(color) = p.effect_color {
                    self.effect_color = color;
                }
                self.sync_picker();
                // everything differs from the keyboard until applied
                self.orig_keys.clear();
                self.active_profile = Some(name.to_string());
                self.selected = None;
                self.status = format!("Loaded profile \"{name}\": press Apply to send it");
            }
            Err(e) => self.status = format!("Could not load profile \"{name}\": {e:#}"),
        }
    }

    fn create_profile(&mut self, raw: &str) {
        let name: String = raw
            .trim()
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
            .collect();
        let name = if name.is_empty() {
            format!("Profile {}", self.profiles.len() + 1)
        } else {
            name
        };
        let path = profiles_dir(&self.path).join(format!("{name}.toml"));
        self.status = match self.write_to(&path) {
            Ok(()) => {
                self.active_profile = Some(name.clone());
                self.refresh_profiles();
                format!("Saved profile \"{name}\"")
            }
            Err(e) => format!("Could not save profile: {e}"),
        };
    }

    fn delete_profile(&mut self) {
        let Some(name) = self.active_profile.take() else {
            return;
        };
        let path = profiles_dir(&self.path).join(format!("{name}.toml"));
        self.status = match std::fs::remove_file(&path) {
            Ok(()) => format!("Deleted profile \"{name}\""),
            Err(e) => format!("Could not delete profile: {e}"),
        };
        self.refresh_profiles();
    }

    /// Switch the key codes to a preset layout, keeping each key's colour.
    fn apply_layout(&mut self, preset: &str) {
        let Ok(p) = parse_config(preset) else { return };
        for k in &mut self.keys {
            if let Some(src) = p.iter().find(|s| s.light_pos == k.light_pos) {
                k.value = src.value.clone();
                k.name = src.name.clone();
            }
        }
        self.status = "Layout changed: press Apply to send it".into();
    }

    /// Write only what was edited: key codes, per-key colours and/or the lighting effect.
    fn apply(&mut self) {
        let keys = self.keys.clone();
        if let Err(e) = validate_keys(&keys) {
            self.status = format!("Apply failed: {e:#}");
            return;
        }
        let changed = |f: &dyn Fn(&Key, &Key) -> bool| {
            keys.iter().any(|k| {
                self.orig_keys
                    .iter()
                    .find(|o| o.light_pos == k.light_pos)
                    .is_none_or(|o| f(k, o))
            })
        };
        let keys_changed = changed(&|k, o| k.value != o.value);
        let colours_changed = changed(&|k, o| k.color != o.color);
        let effect_changed = self.effect_dirty;
        if !(keys_changed || colours_changed || effect_changed) {
            self.status = "Nothing to apply: no edits since load or last apply".into();
            return;
        }
        let effect = self.effect;
        let effect_color = self.effect_color.clone();
        self.status = match connect().and_then(|d| {
            if keys_changed {
                d.set_keys(KeyLayer::Normal, &keys)?;
            }
            if colours_changed {
                d.set_custom_light(&keys)?;
            }
            if effect_changed {
                if effect.supports_color() {
                    d.set_effect_color(effect, &effect_color)?;
                } else {
                    d.set_light_mode(effect)?;
                }
            }
            Ok(())
        }) {
            Ok(()) => {
                self.orig_keys = keys;
                self.effect_dirty = false;
                self.effect_color_dirty = false;
                format!(
                    "Applied: {}",
                    [
                        keys_changed.then_some("keys"),
                        effect_changed.then_some("effect"),
                        colours_changed.then_some("per-key colours"),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(", ")
                )
            }
            Err(e) => format!("Apply failed: {e:#}"),
        };
    }

    fn read_from_keyboard(&mut self) {
        let names = self.keys.clone();
        self.status =
            match connect().and_then(|d| d.fetch_keys_layer(KeyLayer::Normal, &[], &names)) {
                Ok(k) => {
                    self.keys = k;
                    self.orig_keys = self.keys.clone();
                    self.effect_dirty = false;
                    self.effect_color_dirty = false;
                    self.refresh_device();
                    self.selected = None;
                    "Read keymap + colours from keyboard".into()
                }
                Err(e) => format!("Read failed: {e}"),
            };
    }
}

// shadcn/ui "zinc" dark theme, blue accent.
const BG: u32 = 0x09090b;
const CARD: u32 = 0x0c0c0f;
const SURFACE: u32 = 0x18181b;
const BORDER: u32 = 0x27272a;
const MUTED_FG: u32 = 0xa1a1aa;
const FG: u32 = 0xfafafa;
const ACCENT: u32 = 0x3b82f6;
const ACCENT_HOVER: u32 = 0x2563eb;

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Primary,
    Blue,
    Green,
    Amber,
}

fn button(id: &'static str, label: &str, variant: Variant) -> gpui::Stateful<gpui::Div> {
    let (bg, hover, fg, border) = match variant {
        Variant::Primary => (ACCENT, ACCENT_HOVER, FG, ACCENT),
        Variant::Blue => (0x1b2a47, 0x24375c, 0xdbeafe, 0x2f4f86),
        Variant::Green => (0x133324, 0x1a4630, 0xdcfce7, 0x22714a),
        Variant::Amber => (0x3a2a0d, 0x4d3910, 0xfef3c7, 0x8a6416),
    };
    div()
        .id(id)
        .h(px(46.0))
        .px_6()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_base()
        .font_weight(gpui::FontWeight::MEDIUM)
        .hover(move |s| s.bg(rgb(hover)))
        .cursor_pointer()
        .child(label.to_string())
}

fn small_button(id: (&'static str, usize), label: &str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .text_sm()
        .hover(|s| s.bg(rgb(SURFACE)))
        .cursor_pointer()
        .child(label.to_string())
}

fn section(title: &str) -> gpui::Div {
    div()
        .text_xs()
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(MUTED_FG))
        .child(title.to_uppercase())
}

impl Render for Configurator {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // place keys: (key index, x, y, width) in units
        let mut placed: Vec<(usize, f32, f32, f32)> = Vec::new();
        let mut max_x = 0f32;
        for (r, row) in layout().iter().enumerate() {
            let mut x = 0.0;
            for cell in row {
                match cell {
                    Cell::Gap(w) => x += w,
                    Cell::K(lp, w) => {
                        if let Some(i) = self.keys.iter().position(|k| k.light_pos == *lp) {
                            placed.push((i, x, r as f32, *w));
                        }
                        x += w;
                    }
                }
            }
            max_x = max_x.max(x);
        }
        // Matrix positions with no physical key on this ANSI board (e.g. the ISO `#` and `<>`
        // slots) are kept in `self.keys` so Apply writes them back unchanged, but not drawn.
        let rows = 6.0 + FUNCTION_ROW_GAP;
        let mut board = div().relative().w(px(max_x * U)).h(px(rows * U));
        let lighting = self.tab == Tab::Lighting;
        for (i, px_x, px_y, kw) in placed {
            let k = &self.keys[i];
            let c = &k.color;
            let base = if lighting && self.effect.supports_color() {
                if self.effect_color.rainbow {
                    hsv(
                        self.started.elapsed().as_secs_f32() * 80.0 + px_x * 15.0,
                        1.0,
                        1.0,
                    )
                } else {
                    let c = &self.effect_color.color;
                    (c.r, c.g, c.b)
                }
            } else {
                (c.r, c.g, c.b)
            };
            let (lr, lg, lb) = if lighting {
                preview(
                    self.effect,
                    px_x,
                    px_y,
                    self.started.elapsed().as_secs_f32(),
                    base,
                )
            } else {
                base
            };
            let (lr, lg, lb) = if lighting
                && self.effect.supports_color()
                && !self.effect_color.rainbow
                && !matches!(self.effect, Effect::Reaction)
            {
                let brightness = lr.max(lg).max(lb) as f32 / 255.0;
                (
                    (base.0 as f32 * brightness) as u8,
                    (base.1 as f32 * brightness) as u8,
                    (base.2 as f32 * brightness) as u8,
                )
            } else {
                (lr, lg, lb)
            };
            let led_val = ((lr as u32) << 16) | ((lg as u32) << 8) | lb as u32;
            let label = self
                .label_for(&k.value)
                .unwrap_or(&k.name)
                .replace("滚轮", "Knob");
            let selected = self.selected == Some(i);
            let is_knob = k.light_pos == 84;
            let lit = lr.max(lg).max(lb) > 24;
            // keycap face picks up a little of the LED colour while previewing lighting
            let face = if lighting && lit {
                mix(0x232328, led_val, 0.16)
            } else {
                0x232328
            };
            let glow: Hsla = Hsla {
                a: if lit { 0.55 } else { 0.0 },
                ..rgb(led_val).into()
            };
            let (w, h) = (kw * U - 5.0, U - 5.0);
            let mut cap = div()
                .id(("key", i))
                .absolute()
                .left(px(px_x * U + 1.0))
                .top(px((px_y + if px_y > 0.0 { FUNCTION_ROW_GAP } else { 0.0 })
                    * U
                    + 1.0))
                .w(px(if is_knob { h } else { w }))
                .h(px(h))
                .rounded(px(if is_knob { h } else { 6.0 }))
                .bg(rgb(0x0f0f12))
                .border_1()
                .border_color(rgb(if selected { ACCENT } else { 0x2e2e35 }))
                .shadow(vec![BoxShadow {
                    color: glow,
                    offset: point(px(0.0), px(2.0)),
                    blur_radius: px(10.0),
                    spread_radius: px(0.0),
                }])
                .cursor_pointer();
            if selected {
                cap = cap.border_2();
            }
            board = board.child(
                cap.child(
                    // keycap top face, inset so the darker "sides" show around it
                    div()
                        .absolute()
                        .left(px(3.0))
                        .right(px(3.0))
                        .top(px(2.0))
                        .bottom(px(6.0))
                        .rounded(px(if is_knob { 30.0 } else { 4.0 }))
                        .bg(rgb(face))
                        .hover(|s| s.bg(rgb(0x2c2c33)))
                        .text_color(rgb(if selected { FG } else { 0xc4c4cc }))
                        .text_xs()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(label),
                )
                // LED underglow strip along the bottom edge
                .child(
                    div()
                        .absolute()
                        .left(px(6.0))
                        .right(px(6.0))
                        .bottom(px(1.0))
                        .h(px(3.0))
                        .rounded_full()
                        .bg(rgb(led_val)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected = Some(i);
                    this.sync_picker();
                    cx.notify();
                })),
            );
        }

        let sel_color = self.edited_color();

        // ---- colour editor (selected key) ----
        let colour_block = {
            let mut b = div().flex().flex_col().gap_3().min_w(px(280.0));
            if self.editing_effect_color() {
                b = b.child(section("Effect colour (all keys)"))
                    .child(div().flex().gap_2().children([(false, "Single colour"), (true, "Rainbow")].into_iter().map(|(rainbow, label)| {
                        small_button(("effect-colour-mode", usize::from(rainbow)), label)
                            .bg(rgb(if self.effect_color.rainbow == rainbow { 0x16233d } else { SURFACE }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.effect_color.rainbow = rainbow;
                                this.effect_color_dirty = true;
                                this.effect_dirty = true;
                                this.status = "Effect colour mode edited: press Apply".into();
                                cx.notify();
                            }))
                    })))
                    .child(div().text_xs().text_color(rgb(MUTED_FG)).child("Choose a colour for the effect, or Rainbow. The custom per-key colours are kept."));
            } else if self.tab == Tab::Lighting && self.effect != Effect::Custom {
                b = b.child(div().text_sm().text_color(rgb(MUTED_FG)).child("This effect uses its own firmware colours. Choose Custom to edit individual keys."));
            }
            match sel_color {
                None if self.tab != Tab::Lighting || self.effect == Effect::Custom => {
                    b = b.child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED_FG))
                            .child("Select a key to edit its colour."),
                    );
                }
                None => {}
                Some((r, g, bl)) => {
                    let cur = ((r as u32) << 16) | ((g as u32) << 8) | bl as u32;
                    let swatch = |id: &'static str, i: usize, p: u32, on: bool| {
                        div()
                            .id((id, i))
                            .size(px(22.0))
                            .rounded_full()
                            .bg(rgb(p))
                            .border_2()
                            .border_color(rgb(if on { ACCENT } else { BORDER }))
                            .cursor_pointer()
                    };
                    b = b
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size(px(28.0))
                                        .rounded_md()
                                        .bg(rgb(cur))
                                        .border_1()
                                        .border_color(rgb(BORDER)),
                                )
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(format!("#{cur:06X}")),
                                ),
                        )
                        .child(color_picker::render(self.picker, cx))
                        .children(
                            [("R", r, 0usize), ("G", g, 1), ("B", bl, 2)]
                                .into_iter()
                                .map(|(name, val, ch)| {
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .text_sm()
                                        .child(
                                            div().w(px(14.0)).text_color(rgb(MUTED_FG)).child(name),
                                        )
                                        .child(self.slider(
                                            cx,
                                            ["sr", "sg", "sb"][ch],
                                            val as usize * 31 / 255,
                                            31,
                                            move |this, s| {
                                                this.set_channel(ch, (s * 255 / 31) as u8);
                                            },
                                        ))
                                        .child(
                                            div()
                                                .w(px(30.0))
                                                .text_xs()
                                                .text_color(rgb(MUTED_FG))
                                                .child(val.to_string()),
                                        )
                                }),
                        )
                        .child(div().flex().flex_wrap().gap_2().children(
                            PALETTE.iter().enumerate().map(|(i, &p)| {
                                swatch("sw", i, p, cur == p).on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.set_color(p);
                                        cx.notify();
                                    },
                                ))
                            }),
                        ))
                        .child(section("Custom colours"))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .items_center()
                                .children(self.custom.iter().enumerate().map(|(i, &p)| {
                                    swatch("cu", i, p, cur == p).on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.set_color(p);
                                            cx.notify();
                                        },
                                    ))
                                }))
                                .child(small_button(("add", 0), "+").on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if !this.custom.contains(&cur) && this.custom.len() < 10 {
                                            this.custom.push(cur);
                                        }
                                        cx.notify();
                                    },
                                ))),
                        );
                }
            }
            b
        };

        // ---- key function list ----
        let function_block = self.selected.map(|i| {
            let k = &self.keys[i];
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(self.label_for(&k.value).unwrap_or(&k.name).to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(MUTED_FG))
                                .child(k.value.clone()),
                        ),
                )
                .child(section("Function"))
                .child(
                    div()
                        .id("choices")
                        .h(px(220.0))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .children(self.choices.iter().enumerate().map(|(ci, (val, name))| {
                            let is_cur = *val == k.value;
                            div()
                                .id(("choice", ci))
                                .px_2()
                                .py_1()
                                .text_sm()
                                .text_color(rgb(if is_cur { FG } else { MUTED_FG }))
                                .bg(rgb(if is_cur { SURFACE } else { CARD }))
                                .hover(|s| s.bg(rgb(SURFACE)))
                                .cursor_pointer()
                                .child(name.clone())
                                .on_click(cx.listener({
                                    let val = val.clone();
                                    move |this, _, _, cx| {
                                        if let Some(i) = this.selected {
                                            this.keys[i].value = val.clone();
                                        }
                                        cx.notify();
                                    }
                                }))
                        })),
                )
        });

        let card = || {
            div()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(CARD))
        };
        // keyboard case: outer shell, recessed plate, then the keys
        let board_card = div()
            .p(px(14.0))
            .rounded(px(18.0))
            .border_1()
            .border_color(rgb(0x34343c))
            .bg(rgb(0x1b1b20))
            .shadow(vec![BoxShadow {
                color: Hsla {
                    h: 0.0,
                    s: 0.0,
                    l: 0.0,
                    a: 0.6,
                },
                offset: point(px(0.0), px(10.0)),
                blur_radius: px(24.0),
                spread_radius: px(0.0),
            }])
            .child(
                div()
                    .p(px(10.0))
                    .rounded(px(10.0))
                    .bg(rgb(0x0a0a0c))
                    .border_1()
                    .border_color(rgb(0x000000))
                    .child(board),
            );

        // ---- effect picker (Lighting tab): grouped tiles with a colour swatch ----
        let cur_name = EFFECTS
            .iter()
            .find(|(e, _)| *e == self.effect)
            .map_or("", |(_, n)| *n);
        let mut effects_card = card().w_full().flex().flex_col().gap_4().child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(section("Light effect"))
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(cur_name),
                )
                .child(
                    div()
                        .px_2()
                        .rounded_full()
                        .text_xs()
                        .text_color(rgb(if self.effect_dirty {
                            0xfbbf24
                        } else {
                            0x4ade80
                        }))
                        .bg(rgb(if self.effect_dirty {
                            0x3a2a0d
                        } else {
                            0x133324
                        }))
                        .child(if self.effect_dirty {
                            "not applied"
                        } else {
                            "on keyboard"
                        }),
                ),
        );
        for (gi, (group, members)) in EFFECT_GROUPS.iter().enumerate() {
            effects_card = effects_card.child(
                div().flex().flex_col().gap_2().child(section(group)).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .children(members.iter().map(|&ei| {
                            let (eff, name) = EFFECTS[ei];
                            let on = self.effect == eff;
                            div()
                                .id(("effect", gi * 100 + ei))
                                .h(px(52.0))
                                .px_5()
                                .flex()
                                .items_center()
                                .gap_2()
                                .rounded_lg()
                                .border_1()
                                .border_color(rgb(if on { ACCENT } else { BORDER }))
                                .bg(rgb(if on { 0x16233d } else { SURFACE }))
                                .text_base()
                                .text_color(rgb(if on { FG } else { MUTED_FG }))
                                .hover(|s| s.bg(rgb(if on { 0x1b2c4d } else { 0x222227 })))
                                .cursor_pointer()
                                .child(name)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_effect(eff);
                                    cx.notify();
                                }))
                        })),
                ),
            );
        }

        let body = if self.tab == Tab::Lighting {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap_5()
                .child(effects_card)
                .child(board_card)
                .child(
                    card()
                        .flex()
                        .flex_wrap()
                        .justify_center()
                        .gap_8()
                        .child(colour_block)
                        .child(
                            div()
                                .max_w(px(360.0))
                                .text_xs()
                                .text_color(rgb(MUTED_FG))
                                .child("Preview is an approximation of the firmware effect. Apply sends the effect to the keyboard. Brightness and speed controls are coming once the protocol is verified."),
                        ),
                )
        } else {
            let mut editor = card()
                .flex()
                .flex_wrap()
                .justify_center()
                .gap_8()
                .child(colour_block);
            if let Some(f) = function_block {
                editor = editor.child(div().w(px(300.0)).child(f));
            }
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .gap_5()
                .child(board_card)
                .child(editor)
        };

        div()
            .id("root")
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(BG))
            .text_color(rgb(FG))
            .p_6()
            .flex()
            .flex_col()
            .gap_5()
            .child(self.header(cx))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_5()
                    .child(self.sidebar(cx))
                    .child(body),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .text_color(rgb(MUTED_FG))
                    .child(div().size(px(6.0)).rounded_full().bg(rgb(ACCENT)))
                    .child(self.status.clone()),
            )
    }
}

impl Configurator {
    fn header(&self, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex()
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("AULA F75"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED_FG))
                            .child(self.path.display().to_string()),
                    )
                    .child(
                        div().flex().gap_1().mt_2().children(
                            [(Tab::Keys, "Keys"), (Tab::Lighting, "Lighting")]
                                .into_iter()
                                .enumerate()
                                .map(|(ti, (tab, name))| {
                                    let on = self.tab == tab;
                                    div()
                                        .id(("tab", ti))
                                        .px_3()
                                        .py_1()
                                        .rounded_md()
                                        .text_sm()
                                        .bg(rgb(if on { SURFACE } else { BG }))
                                        .text_color(rgb(if on { FG } else { MUTED_FG }))
                                        .hover(|s| s.bg(rgb(SURFACE)))
                                        .cursor_pointer()
                                        .child(name)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.tab = tab;
                                            this.sync_picker();
                                            cx.notify();
                                        }))
                                }),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("conn")
                            .h(px(32.0))
                            .px_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .text_sm()
                            .text_color(rgb(MUTED_FG))
                            .hover(|s| s.bg(rgb(SURFACE)))
                            .cursor_pointer()
                            .child(
                                div()
                                    .size(px(8.0))
                                    .rounded_full()
                                    .bg(rgb(if self.connected { 0x22c55e } else { 0x71717a })),
                            )
                            .child(if self.connected {
                                "Connected (USB)"
                            } else {
                                "Not connected"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_device();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("battery")
                            .h(px(32.0))
                            .px_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .text_sm()
                            .text_color(rgb(MUTED_FG))
                            .hover(|s| s.bg(rgb(SURFACE)))
                            .cursor_pointer()
                            .child(match self.battery {
                                Some((lvl, true)) => format!("\u{26a1} {lvl}%"),
                                Some((lvl, false)) => format!("Battery {lvl}%"),
                                None => "Battery n/a".to_string(),
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_device();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("reload", "Reload", Variant::Blue).on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.reload();
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(button("save", "Save", Variant::Green).on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.save();
                                    cx.notify();
                                },
                            )))
                            .child(
                                button("read", "Read from keyboard", Variant::Amber).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.read_from_keyboard();
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(button("apply", "Apply", Variant::Primary).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.apply();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
    }

    /// Full-width list row used by the sidebar (profiles, layouts).
    fn item(id: (&'static str, usize), label: &str, on: bool) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .w_full()
            .px_3()
            .py_2()
            .rounded_md()
            .text_base()
            .bg(rgb(if on { SURFACE } else { CARD }))
            .text_color(rgb(if on { FG } else { MUTED_FG }))
            .border_l_2()
            .border_color(rgb(if on { ACCENT } else { CARD }))
            .hover(|s| s.bg(rgb(SURFACE)))
            .cursor_pointer()
            .child(label.to_string())
    }

    /// Left sidebar: profile list, and (Keys tab) the key-layout presets.
    fn sidebar(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut profiles = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(section("Profiles"))
            .child(
                Self::item(
                    ("profile", usize::MAX),
                    "Default",
                    self.active_profile.is_none(),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.active_profile = None;
                    this.reload();
                    cx.notify();
                })),
            )
            .children(self.profiles.iter().enumerate().map(|(pi, name)| {
                let n = name.clone();
                Self::item(
                    ("profile", pi),
                    name,
                    self.active_profile.as_deref() == Some(name),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.load_profile(&n);
                    cx.notify();
                }))
            }));
        profiles = match &self.naming {
            Some(text) => profiles.child(
                div()
                    .id("profile-name")
                    .track_focus(&self.name_focus)
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                        let Some(text) = this.naming.as_mut() else {
                            return;
                        };
                        match ev.keystroke.key.as_str() {
                            "enter" => {
                                let name = this.naming.take().unwrap_or_default();
                                this.create_profile(&name);
                            }
                            "escape" => this.naming = None,
                            "backspace" => {
                                text.pop();
                            }
                            _ => {
                                if let Some(c) = &ev.keystroke.key_char {
                                    if !ev.keystroke.modifiers.control {
                                        text.push_str(c);
                                    }
                                }
                            }
                        }
                        cx.notify();
                    }))
                    .w_full()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(ACCENT))
                    .text_sm()
                    .child(if text.is_empty() {
                        "Name, then Enter…".to_string()
                    } else {
                        format!("{text}|")
                    }),
            ),
            None => profiles.child(
                Self::item(("profile-new", 0), "+ New profile", false).on_click(cx.listener(
                    |this, _, window, cx| {
                        this.naming = Some(String::new());
                        window.focus(&this.name_focus);
                        cx.notify();
                    },
                )),
            ),
        };
        if self.active_profile.is_some() {
            profiles = profiles.child(
                Self::item(("profile-del", 0), "Delete this profile", false).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.delete_profile();
                        cx.notify();
                    },
                )),
            );
        }
        let mut side = div()
            .w(px(210.0))
            .flex_none()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(CARD))
            .flex()
            .flex_col()
            .gap_5()
            .child(profiles);
        if self.tab == Tab::Keys {
            side = side.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section("Layout"))
                    .children(LAYOUTS.iter().enumerate().map(|(li, (name, preset))| {
                        let preset = preset.unwrap_or(DEFAULT_CONFIG);
                        Self::item(("layout", li), name, self.layout == Some(li)).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.layout = Some(li);
                                this.apply_layout(preset);
                                cx.notify();
                            }),
                        )
                    })),
            );
        }
        side
    }

    /// Clickable segmented slider: `value` and `max` are segment indices.
    fn slider(
        &self,
        cx: &mut Context<Self>,
        id: &'static str,
        value: usize,
        max: usize,
        on_set: impl Fn(&mut Self, usize) + Clone + 'static,
    ) -> gpui::Div {
        div().flex().gap(px(2.0)).children((0..=max).map(|s| {
            let on_set = on_set.clone();
            div()
                .id((id, s))
                .w(px(if max > 12 { 6.0 } else { 14.0 }))
                .h(px(14.0))
                .rounded_sm()
                .bg(rgb(if s <= value { ACCENT } else { SURFACE }))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    on_set(this, s);
                    cx.notify();
                }))
        }))
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".config/aula-f75/config.toml"))
                .unwrap_or_else(|| PathBuf::from("config.toml"))
        });
    Application::new().run(move |cx: &mut App| {
        let bounds = gpui::Bounds::centered(None, gpui::size(px(1280.0), px(760.0)), cx);
        let opts = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };
        cx.open_window(opts, |window, cx| {
            window.set_window_title("AULA F75 Configurator");
            cx.new(|cx| {
                cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(33))
                            .await;
                        let alive = this.update(cx, |s: &mut Configurator, cx| {
                            if s.tab == Tab::Lighting
                                && !matches!(s.effect, Effect::Custom | Effect::Off)
                            {
                                cx.notify();
                            }
                        });
                        if alive.is_err() {
                            break;
                        }
                    }
                })
                .detach();
                Configurator::new(path, cx)
            })
        })
        .expect("open window");
        cx.activate(true);
    });
}
