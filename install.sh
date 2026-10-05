#!/usr/bin/env bash
# Installer for aula-f75: Debian/Ubuntu, Fedora/RHEL, Arch families. (NixOS: see README, use the flake.)
set -euo pipefail

PREFIX=/usr/local
GUI=1
DEPS=1
UNINSTALL=0

usage() {
    cat <<USAGE
usage: ./install.sh [--prefix DIR] [--cli-only] [--no-deps] [--uninstall]
  --prefix DIR   install under DIR (default /usr/local)
  --cli-only     skip the GPUI configurator (far fewer build dependencies)
  --no-deps      don't install build dependencies via the package manager
  --uninstall    remove everything this script installed
USAGE
}

while [ $# -gt 0 ]; do
    case $1 in
        --prefix) PREFIX=${2:?--prefix needs a value}; shift 2 ;;
        --cli-only) GUI=0; shift ;;
        --no-deps) DEPS=0; shift ;;
        --uninstall) UNINSTALL=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 1 ;;
    esac
done

cd "$(dirname "$(readlink -f "$0")")"

SUDO=
[ "$(id -u)" -eq 0 ] || SUDO=sudo
RULE=/etc/udev/rules.d/70-aula-f75.rules

reload_udev() {
    $SUDO udevadm control --reload-rules
    $SUDO udevadm trigger --subsystem-match=hidraw
}

if [ "$UNINSTALL" -eq 1 ]; then
    $SUDO rm -fv "$PREFIX/bin/aula-f75" "$PREFIX/bin/aula-f75-gui" \
        "$PREFIX/share/applications/aula-f75-gui.desktop" "$PREFIX/share/aula-f75/default.toml" "$RULE"
    $SUDO rmdir --ignore-fail-on-non-empty "$PREFIX/share/aula-f75" 2>/dev/null || true
    reload_udev
    echo "Removed. Your configs in ~/.config/aula-f75 were left alone."
    exit 0
fi

# --- distro detection -------------------------------------------------------
[ -r /etc/os-release ] || { echo "no /etc/os-release; can't detect distro" >&2; exit 1; }
# shellcheck disable=SC1091
. /etc/os-release
family=
for id in $ID ${ID_LIKE:-}; do
    case $id in
        nixos) family=nixos; break ;;
        debian|ubuntu) family=debian; break ;;
        fedora|rhel|centos) family=fedora; break ;;
        arch) family=arch; break ;;
    esac
done

if [ "$family" = nixos ]; then
    cat >&2 <<'MSG'
NixOS can't install udev rules or binaries imperatively. Use the flake:
see "NixOS" in README.md (nixosModules.default sets up the package and the udev rule).
MSG
    exit 1
fi
[ -n "$family" ] || { echo "unsupported distro '$ID'; install deps by hand (see README) and run with --no-deps" >&2; exit 1; }

# --- build dependencies -----------------------------------------------------
if [ "$DEPS" -eq 1 ]; then
    echo "==> Installing build dependencies ($family)"
    case $family in
        debian)
            pkgs=(build-essential pkg-config libudev-dev curl ca-certificates)
            [ "$GUI" -eq 1 ] && pkgs+=(cmake clang libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libvulkan-dev
                mesa-vulkan-drivers libfontconfig-dev libfreetype-dev libssl-dev libxcb1-dev libx11-dev libx11-xcb-dev
                libxcb-xkb-dev libasound2-dev libzstd-dev)
            $SUDO apt-get update
            $SUDO apt-get install -y "${pkgs[@]}" ;;
        fedora)
            pkgs=(gcc gcc-c++ pkgconf-pkg-config systemd-devel curl)
            [ "$GUI" -eq 1 ] && pkgs+=(cmake clang wayland-devel libxkbcommon-devel libxkbcommon-x11-devel vulkan-loader-devel
                mesa-vulkan-drivers fontconfig-devel freetype-devel openssl-devel libxcb-devel libX11-devel alsa-lib-devel
                libzstd-devel)
            $SUDO dnf install -y "${pkgs[@]}" ;;
        arch)
            pkgs=(base-devel pkgconf systemd-libs curl)
            [ "$GUI" -eq 1 ] && pkgs+=(cmake clang wayland libxkbcommon libxkbcommon-x11 vulkan-icd-loader fontconfig freetype2
                openssl libxcb libx11 alsa-lib zstd)
            $SUDO pacman -S --needed --noconfirm "${pkgs[@]}" ;;
    esac
fi

# --- rust -------------------------------------------------------------------
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v cargo >/dev/null; then
    echo "cargo not found. Install Rust from https://rustup.rs (distro packages are usually too old; edition 2024 needs 1.85+)." >&2
    exit 1
fi
minor=$(rustc --version | sed -E 's/rustc 1\.([0-9]+).*/\1/')
if [ "${minor:-0}" -lt 85 ]; then
    echo "rustc is too old ($(rustc --version)); need 1.85+. Run: rustup update stable" >&2
    exit 1
fi

# --- build + install --------------------------------------------------------
echo "==> Building"
if [ "$GUI" -eq 1 ]; then
    cargo build --release --locked --features gui
else
    cargo build --release --locked --bin aula-f75
fi

echo "==> Installing to $PREFIX (and $RULE)"
$SUDO install -Dm755 target/release/aula-f75 "$PREFIX/bin/aula-f75"
$SUDO install -Dm644 data/default.toml "$PREFIX/share/aula-f75/default.toml"
if [ "$GUI" -eq 1 ]; then
    $SUDO install -Dm755 target/release/aula-f75-gui "$PREFIX/bin/aula-f75-gui"
    $SUDO install -Dm644 packaging/aula-f75-gui.desktop "$PREFIX/share/applications/aula-f75-gui.desktop"
fi
$SUDO install -Dm644 packaging/70-aula-f75.rules "$RULE"
reload_udev

gui_hint=
[ "$GUI" -eq 1 ] && gui_hint="   or   aula-f75-gui"
cat <<MSG

Done. Unplug and replug the keyboard (USB cable, not the dongle) if it was already connected.
Then: aula-f75 $PREFIX/share/aula-f75/default.toml$gui_hint
If access is still denied, log out and back in once so the seat ACL is applied.
MSG
