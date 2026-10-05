# aula-f75-linux

Linux tooling for the AULA F75 keyboard (USB `258a:010c`), built on
[Nokkasiili/aula-f75-linux](https://github.com/Nokkasiili/aula-f75-linux) (HID protocol + keymap/light writer).

- `aula-f75 <config.toml>` apply keymap + lighting
- `aula-f75 --dump <names.toml> <out.toml>` read the keyboard's keymap/colours (backup)
- `aula-f75-gui [config.toml]` GPUI configurator (`cargo build --features gui`, dark zinc/blue UI): click a key, pick function/colour, Save / Read from keyboard / Apply

Dev shell: `nix develop`. Needs a hidraw udev rule, e.g.
`SUBSYSTEM=="hidraw", ATTRS{idVendor}=="258a", ATTRS{idProduct}=="010c", MODE="0666"`.
Settings only reach the keyboard over USB, not the 2.4GHz dongle.

`data/default.toml` is the stock keymap + colours. `examples/finnish-ansi.toml` is upstream's Finnish-ANSI sample.
`vendor/xattr` patches a gpui transitive dependency that no longer builds against current libc.
