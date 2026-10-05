# Fn key / Fn+combos stop working (corrupted settings block)

Found and fixed 2026-10-05 on a real F75 (`258a:010c`). Keep this if the symptoms below ever come back.

## Symptoms
- Fn+F12 types plain F12, Fn+↑ is plain ↑, Fn+W/3/4/Esc type W/3/4/Esc: no media keys, no brightness/speed.
- Fn itself still lights W, 3 and 4 and sends nothing on its own, so it *looks* alive.
- Same on USB and 2.4GHz. A factory-reset command, replugging, and re-writing the Fn key value
  (`0x0d000000` vs `0x0d000001`) changed nothing.

## What was NOT wrong (ruled out)
- The Fn key's own mapping: slot 53 = `0x0d000000`, same as stock. (Temporarily mapping slot 53 to `F`
  made the key right of Space type `F`, which confirms slot 53 is the physical Fn key.)
- The keymap layers: layers 0 (Normal) and 1 (Fn, 36 entries) were byte-identical to a healthy F75.
- The connection (USB vs dongle) and the `CMD_RESET` factory-reset command (accepted, changes nothing).

## Root cause
The 128-byte **settings block** (`get_basic_raw`, `CMD_GET_BASIC_INFO`) was corrupted. Diffed against a
working F75 (dumped read-only with `examples/rawdump.rs`):

| byte | healthy | broken |
|---|---|---|
| `[0]` mac-mode flag | `0` | `6` (non-zero = Mac mode, which uses an empty Fn layer 2) |
| `[1]` polling level | `3` | `132` (not a valid value) |
| `[8]` | `7` | `0` |
| `[56..]` light table | clean `(brightness, speed)` pairs | stray `255 0` and `0 55`/`0 68` (brightness 0) entries |

`6, 132` is the report id + `0x84` (`CMD_GET_BASIC_INFO`): the signature of an early version of this
tool that rebuilt the whole block from a few parsed fields plus hardcoded bytes
(`legacy_basic_payload`, which "can clobber settings it does not model"). That is a strong guess, not
something I proved; the byte-level evidence (above) is.

## Fix that worked
Copy the known-good header bytes from a healthy F75 into the broken one, leaving mode and everything else:

```bash
# 1. on a HEALTHY F75 (only it plugged in): read-only dump
nix develop --command cargo run -q --example rawdump > good-settings-block.txt
# 2. on the BROKEN keyboard (only it plugged in): back up, then dry-run, then write
nix develop --command cargo run -q --example rawdump > broken-settings-block.txt
nix develop --command cargo run -q --example blockpatch -- good-settings-block.txt --head        # dry run
nix develop --command cargo run -q --example blockpatch -- good-settings-block.txt --head --yes  # writes bytes 0..9
```

Result: `[0] 6->0  [1] 132->3  [8] 0->7`; Fn+F12 / Fn+↑ worked immediately, no replug needed.

Optional second step (not needed for Fn): drop `--head` to also copy bytes 56..127, which repairs the
light table. That copies the donor's brightness/speed values.

## Roll back
```bash
nix develop --command cargo run -q --example blockpatch -- broken-settings-block.txt --restore
```

## Prevention
- Never write the settings block from parsed fields. Use read-modify-write of the raw block only:
  `AulaF75::set_light_mode` (changes just bytes 9..11) or `get_basic_raw` / `set_basic_raw`.
  `set_basic_info` / `legacy_basic_payload` are kept only for comparison (`examples/diff_basic.rs`).
- Back up before experimenting: `aula-f75 --dump`, `examples/rawdump.rs`, `examples/layers.rs`
  (the GUI's own backups go in `~/.config/aula-f75/backups/`).
- If a keyboard misbehaves, a second working F75 is the best tool: dump both and diff.

## Related open question
Several light modes (ids 1, 9, 14, 16, 18, 19) showed no light in testing. The broken table had
zero-brightness entries, so some of those may only have *looked* dead. Re-test them after the table
repair (`blockpatch` without `--head`).

## Diagnostic tools added (all read-only unless noted)
`rawdump` (settings block), `layers` (all key layers), `capture` (raw HID input reports),
`watch` / `diff_basic` (live byte changes), `dongle_probe` (2.4GHz dongle), `mode_walk`
(writes the light-mode bytes, restores them on exit), `fnkey` (writes one Fn-slot value),
`reset` (factory-reset command; had no visible effect), `blockpatch` (writes, see above).
