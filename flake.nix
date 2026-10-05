{
  description = "AULA F75 Linux: HID driver, CLI and GPUI configurator";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [rustc cargo rust-analyzer clippy rustfmt pkg-config cmake clang];
      buildInputs = with pkgs; [
        udev
        # GPUI (Wayland/X11 + Vulkan + text)
        wayland
        libxkbcommon
        vulkan-loader
        fontconfig
        freetype
        openssl
        libxcb
        libx11
        libxcursor
        libxi
        libxrandr
        alsa-lib
        zstd
      ];
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [wayland libxkbcommon vulkan-loader libx11 libxcursor libxi libxrandr]);
      LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    };
  };
}
