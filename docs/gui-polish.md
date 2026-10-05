# GUI polish verification

Implemented for [issue #4](https://github.com/daschinmoy21/aula-f75-linux/issues/4):

- A hue/saturation colour wheel with a separate colour-brightness strip. Click or
  drag to edit the selected key; the hex swatch, RGB sliders and keyboard colour
  update together. Brightness here scales that key's RGB colour and is independent
  of the firmware effect's brightness setting.
- HSV state retains hue at white/black and saturation at black, so adjusting
  brightness does not discard the chosen hue.
- Function keys now have separate F1–F4, F5–F8 and F9–F12 groups. The function row
  has extra spacing below it, and the right Fn/Ctrl key widths and adjacent gap
  match the ANSI board more closely.

Physical proportions were compared with AULA Gear's
[F75 product photo](https://aulagear.com/cdn/shop/files/203A5822.jpg?v=1734921227&width=1946)
on the [F75 product page](https://aulagear.com/products/aula-f75). This is a visual
approximation, not a measurement of every chassis dimension or LED position.

The GUI was rendered on an isolated Xvfb display with Openbox and Mesa's software
Vulkan driver. Selecting a key, clicking the wheel centre and dragging the
brightness strip were checked visually: the swatch and RGB values updated to the
chosen colour, and the key preview followed the change. Colour conversion and
hue-retention behaviour also have unit tests.

The initial rendering check used only the `3554:fa09` wireless receiver.
Wired custom-light follow-up is described below. Animation fidelity still
requires a comparison with the board. Existing animation
previews remain explicitly approximate; no new firmware effect behaviour is
claimed. See [upstream licensing](upstream-licensing.md) for the remaining licence
follow-up.

## Picker and wired custom-light follow-up

The wheel now uses one cached image instead of thousands of small canvas quads
on every redraw. The brightness strip uses one gradient. Static Custom/Off
previews do not request animation redraws.

Wired custom lighting uses command `0x06`, with separate R/G/B planes of 126
slots. The effect palette (`0x0a`) is a different block and must not be treated
as per-key RGB. Custom mode requires settings payload byte 9 = 1 and byte 10 =
21, followed by the custom-table write. The earlier driver treated these two
bytes as one big-endian effect ID and omitted the enable flag.

This sequence is supported by [Aula-Manager's driver and hardware notes](https://github.com/free-soldier28/Aula-Manager/blob/main/docs/PROTOCOL.md).
On the attached wired F75, the settings and planar RGB read-back checks pass.
The user confirmed that Esc physically displayed green after the controlled
one-key test. Other table bytes were preserved, and the user also observed
blue W/A and arrow keys and a white D. This confirms custom RGB displays on
this board; it does not validate every LED mapping or animation preview.
The driver verifies both blocks after writing and reports failures in the GUI.
