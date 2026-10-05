# Global effect colours

In the Lighting tab, colour-capable firmware effects expose **Single colour** and
**Rainbow** controls. The wheel, brightness strip, RGB sliders and swatches edit
one global effect colour without selecting a key. Editing that colour selects
Single colour; Apply commits it while retaining the chosen effect. Custom remains
the editor for individual per-key colours.

The firmware keeps two different colour blocks:

- Custom per-key RGB: command `0x06`, three 126-byte channel planes.
- Hardware effect RGB profile: command `0x0a`, a 512-byte block. The global colour
  is written to the 98 RGB slots in four regions. Reserved gaps are retained.

A single RGB triplet is insufficient for an all-key effect colour. The four
regions, expressed as payload offsets after removing the eight-byte report
header, are `[21,126)`, `[147,189)`, `[210,294)` and `[315,378)`. The colour write
precedes a settings-read handshake and the effect settings write. The low nibble
of the effect's speed/colour byte selects Single colour (`0`) or Rainbow (`7`);
the speed nibble and brightness remain unchanged.

References: [Aula-Manager](https://github.com/free-soldier28/Aula-Manager/blob/main/docs/PROTOCOL.md)
for effect flags and Custom activation, and
[OpenAula's RGB implementation](https://github.com/not-ayan/openaula/blob/main/src/services/rgbService.ts)
for the separated effect RGB regions and write ordering. These are protocol
references, not proof that every effect works on every firmware revision.

The driver reads both settings and the complete effect profile before writing,
checks their lengths and response headers, verifies read-back, and attempts to
restore the original blocks if applying fails. Settings reads wait for firmware
response preparation. A diagnostic test also checks that the separate Custom
RGB block remains byte-identical.

Reaction was confirmed by the user to light on keypress after changing its colour
flag from `6` to Rainbow (`7`) with brightness already at `9`. The global red
Reaction write passes RGB/settings read-back with Custom colours preserved;
the user confirmed that pressed keys physically light red, and then confirmed
the app worked. Other effects still require
individual hardware checks, particularly the modes marked unverified in the GUI.

Profiles optionally store an `effect_color` table alongside the effect ID. Older
profiles still load. GUI Save/Load and CLI apply retain the global colour and
Rainbow choice; CLI apply now also honours a profile's effect ID. Unsupported
colour/effect combinations are rejected before device writes.

Automatic complete-device backup/restore is still tracked separately in #7.
The diagnostic examples save lighting snapshots in `/tmp`; these are not a
persistent backup history and should not be treated as one.
