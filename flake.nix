{
  description = "AULA F75 Linux: HID driver, CLI and GPUI configurator";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    guiLibs = with pkgs; [wayland libxkbcommon vulkan-loader libx11 libxcursor libxi libxrandr];
    # Source tree minus build output, so editing README etc. doesn't need a full rebuild trigger on target/.
    src = pkgs.lib.cleanSource ./.;
  in {
    packages.${system} = rec {
      default = aula-f75;
      aula-f75 = pkgs.rustPlatform.buildRustPackage {
        pname = "aula-f75";
        version = "0.1.0";
        inherit src;
        cargoLock.lockFile = ./Cargo.lock;
        buildFeatures = ["gui"];
        nativeBuildInputs = with pkgs; [pkg-config cmake clang makeWrapper];
        buildInputs = with pkgs; [udev fontconfig freetype openssl libxcb alsa-lib zstd] ++ guiLibs;
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        # gpui dlopens wayland/vulkan/xkbcommon at runtime.
        postInstall = ''
          wrapProgram $out/bin/aula-f75-gui --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath guiLibs}
          install -Dm644 data/default.toml $out/share/aula-f75/default.toml
          install -Dm644 packaging/aula-f75-gui.desktop $out/share/applications/aula-f75-gui.desktop
          install -Dm644 packaging/70-aula-f75.rules $out/lib/udev/rules.d/70-aula-f75.rules
        '';
        doCheck = false;
        meta.mainProgram = "aula-f75-gui";
      };
    };

    # programs.aula-f75.enable = true;  -> binaries on PATH + udev rule installed
    nixosModules.default = {
      config,
      lib,
      pkgs,
      ...
    }: {
      options.programs.aula-f75.enable = lib.mkEnableOption "AULA F75 keyboard tools and udev access rule";
      config = lib.mkIf config.programs.aula-f75.enable {
        environment.systemPackages = [self.packages.${pkgs.stdenv.hostPlatform.system}.default];
        services.udev.packages = [self.packages.${pkgs.stdenv.hostPlatform.system}.default];
      };
    };

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
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiLibs;
      LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    };
  };
}
