use super::{Configurator, hsv};
use gpui::{
    Bounds, Context, Div, MouseButton, Pixels, Point, RenderImage, canvas, div, fill,
    linear_color_stop, linear_gradient, point, prelude::*, px, rgb, size,
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, OnceLock},
};

const WHEEL_SIZE: f32 = 180.0;
const INSET: f32 = 6.0;

/// Keep HSV state independently of quantized RGB, including hue at white and black.
#[derive(Clone, Copy)]
pub(super) struct ColorPicker {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self {
            hue: 0.0,
            saturation: 1.0,
            value: 1.0,
        }
    }
}

impl ColorPicker {
    pub fn sync_rgb(&mut self, r: u8, g: u8, b: u8) {
        let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        self.value = max;
        if max > 0.0 {
            self.saturation = delta / max;
        }
        if delta > 0.0 {
            self.hue = (60.0
                * if max == r {
                    (g - b) / delta
                } else if max == g {
                    (b - r) / delta + 2.0
                } else {
                    (r - g) / delta + 4.0
                })
            .rem_euclid(360.0);
        }
    }

    /// Normalized wheel coordinates: centre is white, edge fully saturated.
    fn pick(&mut self, x: f32, y: f32) {
        self.saturation = x.hypot(y).min(1.0);
        if self.saturation > 0.0 {
            self.hue = y.atan2(x).to_degrees().rem_euclid(360.0);
        }
    }

    pub fn rgb(self) -> (u8, u8, u8) {
        hsv(self.hue, self.saturation, self.value)
    }
}

fn packed((r, g, b): (u8, u8, u8)) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Generated once at 2x resolution, uploaded once to GPUI's image atlas.
fn wheel_image() -> Arc<RenderImage> {
    static IMAGE: OnceLock<Arc<RenderImage>> = OnceLock::new();
    IMAGE
        .get_or_init(|| {
            let width = (WHEEL_SIZE * 2.0) as u32;
            let centre = width as f32 / 2.0;
            let radius = centre - INSET * 2.0;
            let buffer = image::RgbaImage::from_fn(width, width, |x, y| {
                let (dx, dy) = (x as f32 + 0.5 - centre, y as f32 + 0.5 - centre);
                let distance = dx.hypot(dy);
                let alpha = (radius + 0.5 - distance).clamp(0.0, 1.0);
                let (r, g, b) = hsv(dy.atan2(dx).to_degrees(), (distance / radius).min(1.0), 1.0);
                // RenderImage accepts BGRA pixels, even though the buffer type is RgbaImage.
                image::Rgba([b, g, r, (alpha * 255.0).round() as u8])
            });
            Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
        })
        .clone()
}

fn wheel_point(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let radius = f32::from(bounds.size.width) / 2.0 - INSET;
    let centre = bounds.center();
    (
        f32::from(position.x - centre.x) / radius,
        f32::from(position.y - centre.y) / radius,
    )
}

fn pick_wheel(
    this: &mut Configurator,
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    dragging: bool,
) {
    let (x, y) = wheel_point(position, bounds);
    if dragging || x.hypot(y) <= 1.0 {
        this.picker.pick(x, y);
        this.update_picker_color();
    }
}

fn pick_value(this: &mut Configurator, position: Point<Pixels>, bounds: Bounds<Pixels>) {
    this.picker.value = (1.0
        - f32::from(position.y - bounds.top()) / f32::from(bounds.size.height))
    .clamp(0.0, 1.0);
    this.update_picker_color();
}

pub(super) fn render(picker: ColorPicker, cx: &mut Context<Configurator>) -> Div {
    let wheel_bounds = Rc::new(Cell::new(Bounds::default()));
    let prepaint_bounds = wheel_bounds.clone();
    let down_bounds = wheel_bounds.clone();
    let wheel = div()
        .id("colour-wheel")
        .size(px(WHEEL_SIZE))
        .cursor_crosshair()
        .child(
            canvas(
                move |bounds, _, _| prepaint_bounds.set(bounds),
                move |bounds, _, window, _| {
                    let radius = f32::from(bounds.size.width) / 2.0 - INSET;
                    let centre = bounds.center();
                    window
                        .paint_image(bounds, Default::default(), wheel_image(), 0, false)
                        .expect("colour wheel image");
                    let angle = picker.hue.to_radians();
                    let marker = centre
                        + point(
                            px(angle.cos() * picker.saturation * radius),
                            px(angle.sin() * picker.saturation * radius),
                        );
                    let outer = fill(
                        Bounds::new(marker - point(px(5.0), px(5.0)), size(px(10.0), px(10.0))),
                        rgb(0xffffff),
                    )
                    .corner_radii(px(5.0));
                    let inner = fill(
                        Bounds::new(marker - point(px(3.0), px(3.0)), size(px(6.0), px(6.0))),
                        rgb(0x111111),
                    )
                    .corner_radii(px(3.0));
                    window.paint_quad(outer);
                    window.paint_quad(inner);
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                pick_wheel(this, event.position, down_bounds.get(), false);
                cx.notify();
            }),
        )
        .on_mouse_move(
            cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    pick_wheel(this, event.position, wheel_bounds.get(), true);
                    cx.notify();
                }
            }),
        );

    let value_bounds = Rc::new(Cell::new(Bounds::default()));
    let prepaint_bounds = value_bounds.clone();
    let down_bounds = value_bounds.clone();
    let value = div()
        .id("colour-value")
        .w(px(24.0))
        .h(px(WHEEL_SIZE - INSET * 2.0))
        .cursor_crosshair()
        .child(
            canvas(
                move |bounds, _, _| prepaint_bounds.set(bounds),
                move |bounds, _, window, _| {
                    window.paint_quad(fill(
                        bounds,
                        linear_gradient(
                            180.0,
                            linear_color_stop(
                                rgb(packed(hsv(picker.hue, picker.saturation, 1.0))),
                                0.0,
                            ),
                            linear_color_stop(rgb(0x000000), 1.0),
                        ),
                    ));
                    let y = bounds.size.height * (1.0 - picker.value);
                    window.paint_quad(fill(
                        Bounds::new(
                            bounds.origin + point(px(0.0), y - px(2.0)),
                            size(bounds.size.width, px(4.0)),
                        ),
                        rgb(0x111111),
                    ));
                    window.paint_quad(fill(
                        Bounds::new(
                            bounds.origin + point(px(0.0), y - px(1.0)),
                            size(bounds.size.width, px(2.0)),
                        ),
                        rgb(0xffffff),
                    ));
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                pick_value(this, event.position, down_bounds.get());
                cx.notify();
            }),
        )
        .on_mouse_move(
            cx.listener(move |this, event: &gpui::MouseMoveEvent, _, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    pick_value(this, event.position, value_bounds.get());
                    cx.notify();
                }
            }),
        );

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(wheel)
                .child(value),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(super::MUTED_FG))
                .child(format!(
                    "Hue / saturation · Colour brightness {:.0}%",
                    picker.value * 100.0
                )),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_and_rgb_stay_in_sync() {
        for (point, expected) in [
            ((1.0, 0.0), (255, 0, 0)),
            ((0.0, 0.0), (255, 255, 255)),
            ((-1.0, 0.0), (0, 255, 255)),
        ] {
            let mut picker = ColorPicker::default();
            picker.pick(point.0, point.1);
            assert_eq!(picker.rgb(), expected);
        }
        for (r, g, b) in [
            (20, 134, 243),
            (255, 255, 255),
            (0, 0, 0),
            (0, 255, 0),
            (100, 20, 42),
        ] {
            let mut picker = ColorPicker::default();
            picker.sync_rgb(r, g, b);
            let (pr, pg, pb) = picker.rgb();
            assert!((pr as i16 - r as i16).abs() <= 1);
            assert!((pg as i16 - g as i16).abs() <= 1);
            assert!((pb as i16 - b as i16).abs() <= 1);
        }
    }

    #[test]
    fn black_retains_hue_and_saturation_for_brightness_changes() {
        let mut picker = ColorPicker::default();
        picker.sync_rgb(0, 0, 255);
        picker.sync_rgb(0, 0, 0);
        assert_eq!(picker.rgb(), (0, 0, 0));
        picker.value = 1.0;
        assert_eq!(picker.rgb(), (0, 0, 255));
        picker.pick(2.0, 0.0);
        assert_eq!(picker.rgb(), (255, 0, 0));
    }
}
