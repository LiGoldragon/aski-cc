{
  description = "aski-cc — aski compiler: Surface DB, macro expansion, aski-to-kernel pipeline";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";

    # Local dependency: aski-rs provides the Rust backend
    aski-rs-src = {
      url = "path:../aski-rs";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, fenix, crane, aski-rs-src, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      toolchain = fenix.packages.${system}.stable.toolchain;
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

      # aski-cc source includes the aski/ directory with .aski files
      src = pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
          (craneLib.filterCargoSources path type)
          || (builtins.match ".*\\.aski$" path != null);
      };

      commonArgs = {
        inherit src;
        pname = "aski-cc";
        version = "0.1.0";
        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [ sqlite ];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      aski-cc = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
    in {
      packages.${system}.default = aski-cc;
      devShells.${system}.default = craneLib.devShell {
        packages = with pkgs; [ rust-analyzer pkg-config sqlite ];
      };
    };
}
