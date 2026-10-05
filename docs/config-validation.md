# Configuration validation and partial writes

Both CLI config loading and GUI profile loading validate the entire key list.
The library's keymap and custom-light setters also validate inputs supplied directly.
A validation error aborts before any device mutation, including when an invalid key
belongs to a different layer from the one being written.

- `value`: 1–8 hexadecimal digits, with an optional `0x` or `0X` prefix. Zero is a
  valid explicit unmapping; invalid strings and overflowing values are errors.
- `pos`: a byte offset divisible by four, from 0 through 500. The keymap command
  declares 504 bytes (126 four-byte slots).
- `light_pos`: from 0 through 125, within each of the RGB channel planes.
- Key positions and lighting positions must each be unique within a layer. Different
  layers may reuse the same positions. Supported layers are 0, 1, and 2.
- Only `key_type = "basic"` is supported. `macro` keys and populated `macro_data`
  are rejected until macro support is implemented.
- Profile effect IDs must be representable by the driver's current `Effect` enum.
  This validates the numeric range, not the observed behaviour of unverified modes.

The driver reads the current target layer before writing and requires exactly 504
keymap bytes. Failed or malformed reads abort without sending a set command. It
patches only the requested four-byte slots and retains every other byte of the map.
Custom lighting follows the same read/patch/write procedure for its 384-byte block,
retaining untouched RGB slots and the six reserved bytes after the channel planes.
Empty updates and updates without keys in the target layer perform no write.

Saved configurations use numeric layer IDs, matching the input format. This fixes
the older serializer's enum-name output, which could not be loaded again.

Run the transport-free regression tests with:

```sh
nix develop --command cargo test --features gui
```

These checks cover validation before I/O, failed/malformed reads without writes,
untouched-byte preservation in all supported layers, partial lighting, config
round trips, and the GUI colour picker's HSV/RGB conversions. The custom-light setter also checks RGB and settings read-back on hardware;
read-back alone does not prove the physical LEDs display the colour. Applying keymaps and
lighting is not an atomic transaction across separate HID commands.
