# aula-f75-linux

Linux tooling for the AULA F75 keyboard (USB `258a:010c`).

> **Credit:** this project exists because of [Nokkasiili/aula-f75-linux](https://github.com/Nokkasiili/aula-f75-linux),
> which reverse-engineered the F75's HID protocol and wrote the original keymap/light driver that everything
> here is built on. Thank you. See [Credits](#credits).

- `aula-f75 <config.toml>` apply keymap + lighting
- `aula-f75 --dump <names.toml> <out.toml>` read the keyboard's keymap/colours (backup)
- `aula-f75-gui [config.toml]` GPUI configurator (`cargo build --features gui`, dark zinc/blue UI): click a key, pick function/colour, Save / Read from keyboard / Apply

Dev shell: `nix develop`. Needs a hidraw udev rule, e.g.
`SUBSYSTEM=="hidraw", ATTRS{idVendor}=="258a", ATTRS{idProduct}=="010c", MODE="0666"`.
Settings only reach the keyboard over USB, not the 2.4GHz dongle.

`data/default.toml` is the stock keymap + colours. `examples/finnish-ansi.toml` is upstream's Finnish-ANSI sample.
`vendor/xattr` patches a gpui transitive dependency that no longer builds against current libc.

## Troubleshooting
- Fn+F-keys / Fn+arrows do nothing: see [docs/fn-layer-fix.md](docs/fn-layer-fix.md) (corrupted settings block, fixed with `examples/blockpatch.rs`).
- Settings can't be changed over the 2.4GHz dongle (`3554:fa09`): it answers with a fixed stub instead of relaying the wired protocol (`examples/dongle_probe.rs`). Use the USB cable.
- Backups of your keymap/settings go in `~/.config/aula-f75/backups/`; take one before experimenting.

## Credits
- [Nokkasiili/aula-f75-linux](https://github.com/Nokkasiili/aula-f75-linux): the original Linux driver. The HID protocol
  (feature reports, keymap and light commands), the key/colour data model and the Finnish-ANSI sample all come from it.
  This repo extends it with a library/CLI split, a GPUI configurator, profiles and diagnostics.
- [xntebli/aula-f75-linux](https://github.com/xntebli/aula-f75-linux): documents running the official Windows configurator
  under Wine; it confirmed that settings need the USB cable, not the 2.4GHz dongle.
