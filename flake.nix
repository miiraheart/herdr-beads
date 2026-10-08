{
  description = "herdr-beads — a beads (bd) board for herdr (List / Table / Kanban, docked or floating)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    packages = forAllSystems (pkgs: rec {
      # The bare TUI binary.
      herdr-beads-bin = pkgs.rustPlatform.buildRustPackage {
        pname = "herdr-beads";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
      };

      # A plugin directory ready to register with herdr (`plugin_root`):
      #   - herdr-plugin.toml with the [[build]] hook removed. herdr runs that
      #     hook (`cargo build --release`) in the plugin root; a Nix store path
      #     is read-only and the binary is already built, so it's dropped here.
      #   - the launcher scripts the manifest's actions exec.
      #   - the binary at ./target/release/herdr-beads, where the manifest's
      #     pane commands look for it.
      herdr-beads = pkgs.runCommand "herdr-beads-plugin" {} ''
        mkdir -p $out/scripts $out/target/release
        cp ${self}/scripts/*.sh $out/scripts/
        cp ${herdr-beads-bin}/bin/herdr-beads $out/target/release/herdr-beads
        ${pkgs.gawk}/bin/awk 'BEGIN{s=0} /^\[\[build\]\]/{s=1} s==1 && /^\[\[/ && !/^\[\[build\]\]/{s=0} s==0{print}' \
          ${self}/herdr-plugin.toml > $out/herdr-plugin.toml
      '';

      default = herdr-beads;
    });

    overlays.default = _final: prev: {
      herdr-beads = self.packages.${prev.stdenv.hostPlatform.system}.herdr-beads;
    };
  };
}
