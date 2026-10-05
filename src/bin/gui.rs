//! GPUI configurator: click a key, pick its function and colour, then apply to the keyboard.
use aula_f75::types::{Color, Effect, Key, KeyLayer};
use aula_f75::{DEFAULT_CONFIG, connect, parse_config, serialize_config};
use gpui::{
    App, Application, Context, Window, WindowOptions, div, prelude::*, px, rgb,
};
use std::path::PathBuf;

/// Matrix is column-major with 6 rows per column (light_pos = col * 6 + row).
const ROWS: usize = 6;
const KEY_W: f32 = 46.0;
const KEY_H: f32 = 46.0;

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
        (0x28, "Enter"), (0x29, "Esc"), (0x2a, "Backspace"), (0x2b, "Tab"), (0x2c, "Space"),
        (0x2d, "-"), (0x2e, "="), (0x2f, "["), (0x30, "]"), (0x31, "\\"), (0x33, ";"),
        (0x34, "'"), (0x35, "`"), (0x36, ","), (0x37, "."), (0x38, "/"), (0x39, "Caps Lock"),
        (0x46, "PrtSc"), (0x47, "ScrLk"), (0x48, "Pause"), (0x49, "Insert"), (0x4a, "Home"),
        (0x4b, "PgUp"), (0x4c, "Delete"), (0x4d, "End"), (0x4e, "PgDn"), (0x4f, "Right"),
        (0x50, "Left"), (0x51, "Down"), (0x52, "Up"),
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
        (0x00010000u32, "L Ctrl"), (0x00020000, "L Shift"), (0x00040000, "L Alt"),
        (0x00080000, "L Win"), (0x00100000, "R Ctrl"), (0x00200000, "R Shift"),
        (0x00400000, "R Alt"), (0x0d000000, "Fn"),
    ] {
        out.push((format!("0x{code:08x}"), name.to_string()));
    }
    out
}

const PALETTE: [u32; 10] = [
    0xff0000, 0xff7f00, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0x8b00ff, 0xff00ff, 0xffffff,
    0x000000,
];

struct Configurator {
    keys: Vec<Key>,
    selected: Option<usize>,
    path: PathBuf,
    status: String,
    choices: Vec<(String, String)>,
}

impl Configurator {
    fn new(path: PathBuf) -> Self {
        let (keys, status) = match std::fs::read_to_string(&path).map(|s| parse_config(&s)) {
            Ok(Ok(k)) => (k, format!("Loaded {}", path.display())),
            _ => (
                parse_config(DEFAULT_CONFIG).expect("bundled default config"),
                format!("{} not found: showing bundled defaults", path.display()),
            ),
        };
        Self { keys, selected: None, path, status, choices: choices() }
    }

    fn label_for(&self, value: &str) -> Option<&str> {
        self.choices.iter().find(|(v, _)| v == value).map(|(_, n)| n.as_str())
    }

    fn set_color(&mut self, rgb_val: u32) {
        if let Some(i) = self.selected {
            let (r, g, b) = ((rgb_val >> 16) as u8, (rgb_val >> 8) as u8, rgb_val as u8);
            self.keys[i].color = Color::create(r, g, b);
        }
    }

    fn nudge(&mut self, ch: usize, delta: i16) {
        if let Some(i) = self.selected {
            let c = &mut self.keys[i].color;
            let f = |x: u8| (x as i16 + delta).clamp(0, 255) as u8;
            let (r, g, b) = match ch {
                0 => (f(c.r), c.g, c.b),
                1 => (c.r, f(c.g), c.b),
                _ => (c.r, c.g, f(c.b)),
            };
            *c = Color::create(r, g, b);
        }
    }

    fn save(&mut self) {
        self.status = match serialize_config(&self.keys)
            .and_then(|s| Ok(std::fs::write(&self.path, s)?))
        {
            Ok(()) => format!("Saved {}", self.path.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    fn apply(&mut self) {
        let keys = self.keys.clone();
        self.status = match connect().and_then(|d| {
            d.set_keys(KeyLayer::Normal, &keys)?;
            let mut info = d.get_basic_info()?;
            info.light_mode = Effect::FixedOn;
            d.set_basic_info(&info)?;
            d.set_custom_light(&keys)?;
            d.set_light_color()
        }) {
            Ok(()) => "Applied to keyboard".into(),
            Err(e) => format!("Apply failed: {e}"),
        };
    }

    fn read_from_keyboard(&mut self) {
        let names = self.keys.clone();
        self.status = match connect().and_then(|d| d.fetch_keys_layer(KeyLayer::Normal, &[], &names)) {
            Ok(k) => {
                self.keys = k;
                self.selected = None;
                "Read keymap + colours from keyboard".into()
            }
            Err(e) => format!("Read failed: {e}"),
        };
    }
}

fn button(id: &'static str, label: &str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_md()
        .bg(rgb(0x3b4252))
        .hover(|s| s.bg(rgb(0x4c566a)))
        .cursor_pointer()
        .child(label.to_string())
}

impl Render for Configurator {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cols = self.keys.iter().map(|k| k.light_pos / ROWS).max().unwrap_or(0) + 1;
        let board_w = cols as f32 * KEY_W;
        let board_h = ROWS as f32 * KEY_H;

        let mut board = div().relative().w(px(board_w)).h(px(board_h));
        for (i, k) in self.keys.iter().enumerate() {
            let (col, row) = (k.light_pos / ROWS, k.light_pos % ROWS);
            let c = &k.color;
            let bg = rgb(((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32);
            let lum = 0.3 * c.r as f32 + 0.59 * c.g as f32 + 0.11 * c.b as f32;
            let fg = if lum > 140.0 { rgb(0x000000) } else { rgb(0xffffff) };
            let label = self.label_for(&k.value).unwrap_or(&k.name).to_string();
            let selected = self.selected == Some(i);
            board = board.child(
                div()
                    .id(("key", i))
                    .absolute()
                    .left(px(col as f32 * KEY_W))
                    .top(px(row as f32 * KEY_H))
                    .w(px(KEY_W - 3.0))
                    .h(px(KEY_H - 3.0))
                    .rounded_md()
                    .bg(bg)
                    .text_color(fg)
                    .text_xs()
                    .border_2()
                    .border_color(if selected { rgb(0x88c0d0) } else { rgb(0x2e3440) })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = Some(i);
                        cx.notify();
                    })),
            );
        }

        let mut panel = div().flex().flex_col().gap_2().w(px(300.0));
        if let Some(i) = self.selected {
            let k = &self.keys[i];
            panel = panel
                .child(format!("Key: {}   (matrix {}, {})", k.name, k.light_pos / ROWS, k.light_pos % ROWS))
                .child(format!("Code: {}", k.value))
                .child("Colour")
                .child(div().flex().flex_wrap().gap_1().children(PALETTE.iter().map(|&p| {
                    div()
                        .id(("sw", p as usize))
                        .size(px(24.0))
                        .rounded_sm()
                        .bg(rgb(p))
                        .border_1()
                        .border_color(rgb(0x4c566a))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_color(p);
                            cx.notify();
                        }))
                })))
                .child(div().flex().gap_2().children(["R", "G", "B"].iter().enumerate().map(
                    |(ch, name)| {
                        div()
                            .flex()
                            .gap_1()
                            .child(*name)
                            .child(button(["r-", "g-", "b-"][ch], "-").on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.nudge(ch, -16);
                                    cx.notify();
                                },
                            )))
                            .child(button(["r+", "g+", "b+"][ch], "+").on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.nudge(ch, 16);
                                    cx.notify();
                                },
                            )))
                    },
                )))
                .child("Function")
                .child(
                    div()
                        .id("choices")
                        .h(px(220.0))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .children(self.choices.iter().enumerate().map(|(ci, (val, name))| {
                            let is_cur = *val == k.value;
                            div()
                                .id(("choice", ci))
                                .px_2()
                                .bg(if is_cur { rgb(0x434c5e) } else { rgb(0x2e3440) })
                                .hover(|s| s.bg(rgb(0x4c566a)))
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
                );
        } else {
            panel = panel.child("Select a key");
        }

        div()
            .size_full()
            .bg(rgb(0x242933))
            .text_color(rgb(0xeceff4))
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button("save", "Save file").on_click(cx.listener(|this, _, _, cx| {
                        this.save();
                        cx.notify();
                    })))
                    .child(button("read", "Read from keyboard").on_click(cx.listener(|this, _, _, cx| {
                        this.read_from_keyboard();
                        cx.notify();
                    })))
                    .child(button("apply", "Apply to keyboard").on_click(cx.listener(|this, _, _, cx| {
                        this.apply();
                        cx.notify();
                    }))),
            )
            .child(div().flex().gap_6().child(board).child(panel))
            .child(div().text_sm().text_color(rgb(0x81a1c1)).child(self.status.clone()))
    }
}

fn main() {
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config/aula-f75/config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    });
    Application::new().run(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Configurator::new(path)))
            .expect("open window");
        cx.activate(true);
    });
}
