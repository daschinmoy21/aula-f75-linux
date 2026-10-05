//! GPUI configurator: click a key, pick its function and colour, then apply to the keyboard.
use aula_f75::types::{Color, Effect, Key, KeyLayer};
use aula_f75::{DEFAULT_CONFIG, connect, parse_config, serialize_config};
use gpui::{
    App, Application, Context, Window, WindowOptions, div, prelude::*, px, rgb,
};
use std::path::PathBuf;

/// Key unit in px. Physical layout below is keyed by `light_pos` (matrix index).
const U: f32 = 50.0;

enum Cell {
    Gap(f32),
    K(usize, f32),
}

/// F75 physical layout: rows of (light_pos, width in units) with gaps.
fn layout() -> Vec<Vec<Cell>> {
    use Cell::{Gap, K};
    let ks = |ps: &[usize], w: f32| -> Vec<Cell> { ps.iter().map(|&p| K(p, w)).collect() };
    let cat = |parts: Vec<Vec<Cell>>| -> Vec<Cell> { parts.into_iter().flatten().collect() };
    vec![
        cat(vec![ks(&[0], 1.0), vec![Gap(1.0)], ks(&[12, 18, 24, 30, 36, 42, 48, 54, 60, 66, 72, 78], 1.0), vec![Gap(1.0)], ks(&[84], 1.0)]),
        cat(vec![ks(&[1, 7, 13, 19, 25, 31, 37, 43, 49, 55, 61, 67, 73], 1.0), ks(&[79], 2.0), ks(&[85], 1.0)]),
        cat(vec![ks(&[2], 1.5), ks(&[8, 14, 20, 26, 32, 38, 44, 50, 56, 62, 68, 74], 1.0), ks(&[80], 1.5), ks(&[86], 1.0)]),
        cat(vec![ks(&[3], 1.75), ks(&[9, 15, 21, 27, 33, 39, 45, 51, 57, 63, 69], 1.0), ks(&[81], 2.25), ks(&[87], 1.0)]),
        cat(vec![ks(&[4], 2.25), ks(&[10, 16, 22, 28, 34, 40, 46, 52, 58, 64], 1.0), ks(&[70], 1.75), ks(&[82, 88], 1.0)]),
        cat(vec![ks(&[5, 11, 17], 1.25), ks(&[35], 6.25), ks(&[53, 59], 1.0), vec![Gap(1.0)], ks(&[77, 83, 89], 1.0)]),
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
        (0x28, "Enter"), (0x29, "Esc"), (0x2a, "Bksp"), (0x2b, "Tab"), (0x2c, "Space"),
        (0x2d, "-"), (0x2e, "="), (0x2f, "["), (0x30, "]"), (0x31, "\\"), (0x33, ";"),
        (0x34, "'"), (0x35, "`"), (0x36, ","), (0x37, "."), (0x38, "/"), (0x39, "Caps"),
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
            .and_then(|s| {
                if let Some(dir) = self.path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                Ok(std::fs::write(&self.path, s)?)
            })
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
    Outline,
}

fn button(id: &'static str, label: &str, variant: Variant) -> gpui::Stateful<gpui::Div> {
    let (bg, hover, fg, border) = match variant {
        Variant::Primary => (ACCENT, ACCENT_HOVER, FG, ACCENT),
        Variant::Outline => (BG, SURFACE, FG, BORDER),
    };
    div()
        .id(id)
        .h(px(32.0))
        .px_3()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_sm()
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
        // keys the layout doesn't know about go in an extra row
        let mut extra_x = 0.0;
        for i in 0..self.keys.len() {
            if !placed.iter().any(|p| p.0 == i) {
                placed.push((i, extra_x, 6.5, 1.0));
                extra_x += 1.0;
            }
        }
        let rows = if extra_x > 0.0 { 7.5 } else { 6.0 };
        let mut board = div().relative().w(px(max_x.max(extra_x) * U)).h(px(rows * U));
        for (i, px_x, px_y, kw) in placed {
            let k = &self.keys[i];
            let c = &k.color;
            let led = rgb(((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32);
            let label = self.label_for(&k.value).unwrap_or(&k.name).replace("滚轮", "Knob");
            let selected = self.selected == Some(i);
            board = board.child(
                div()
                    .id(("key", i))
                    .absolute()
                    .left(px(px_x * U))
                    .top(px(px_y * U))
                    .w(px(kw * U - 4.0))
                    .h(px(U - 4.0))
                    .rounded_md()
                    .bg(rgb(SURFACE))
                    .border_1()
                    .border_color(rgb(if selected { ACCENT } else { BORDER }))
                    .text_color(rgb(if selected { FG } else { MUTED_FG }))
                    .text_xs()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .hover(|s| s.bg(rgb(0x1f1f23)))
                    .cursor_pointer()
                    .child(div().flex_1().flex().items_center().child(label))
                    // colour "LED" strip along the bottom edge
                    .child(div().w_full().h(px(3.0)).bg(led))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = Some(i);
                        cx.notify();
                    })),
            );
        }

        let mut panel = div()
            .w(px(300.0))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(CARD));

        if let Some(i) = self.selected {
            let k = &self.keys[i];
            let c = &k.color;
            panel = panel
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
                .child(section("Colour"))
                .child(div().flex().flex_wrap().gap_2().children(PALETTE.iter().map(|&p| {
                    let on = ((c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32) == p;
                    div()
                        .id(("sw", p as usize))
                        .size(px(22.0))
                        .rounded_full()
                        .bg(rgb(p))
                        .border_2()
                        .border_color(rgb(if on { ACCENT } else { BORDER }))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_color(p);
                            cx.notify();
                        }))
                })))
                .child(div().flex().flex_col().gap_1().children(
                    [("R", c.r), ("G", c.g), ("B", c.b)].into_iter().enumerate().map(
                        |(ch, (name, val))| {
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_sm()
                                .child(div().w(px(14.0)).text_color(rgb(MUTED_FG)).child(name))
                                .child(small_button((["r-", "g-", "b-"][ch], 0), "−").on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.nudge(ch, -16);
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .h(px(4.0))
                                        .rounded_full()
                                        .bg(rgb(SURFACE))
                                        .child(
                                            div()
                                                .h_full()
                                                .rounded_full()
                                                .bg(rgb(ACCENT))
                                                .w(gpui::relative(val as f32 / 255.0)),
                                        ),
                                )
                                .child(small_button((["r+", "g+", "b+"][ch], 0), "+").on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.nudge(ch, 16);
                                        cx.notify();
                                    }),
                                ))
                                .child(
                                    div()
                                        .w(px(28.0))
                                        .text_xs()
                                        .text_color(rgb(MUTED_FG))
                                        .child(val.to_string()),
                                )
                        },
                    ),
                ))
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
                );
        } else {
            panel = panel.child(
                div().text_sm().text_color(rgb(MUTED_FG)).child("Select a key to edit its colour and function."),
            );
        }

        div()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(FG))
            .p_6()
            .flex()
            .flex_col()
            .gap_5()
            .child(
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
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(button("save", "Save", Variant::Outline).on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.save();
                                    cx.notify();
                                },
                            )))
                            .child(
                                button("read", "Read from keyboard", Variant::Outline).on_click(
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
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_5()
                    .child(
                        div()
                            .p_4()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(CARD))
                            .child(board),
                    )
                    .child(panel),
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

fn main() {
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config/aula-f75/config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    });
    Application::new().run(move |cx: &mut App| {
        let bounds = gpui::Bounds::centered(None, gpui::size(px(1280.0), px(600.0)), cx);
        let opts = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            ..Default::default()
        };
        cx.open_window(opts, |_, cx| cx.new(|_| Configurator::new(path)))
            .expect("open window");
        cx.activate(true);
    });
}
