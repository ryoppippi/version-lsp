{
  description = "version-lsp - LSP for package version management";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    let
      cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { pkgs, system, ... }:
        let
          fenixPkgs = inputs.fenix.packages.${system};
          rustToolchain = fenixPkgs.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-sqSWJDUxc+zaz1nBWMAJKTAGBuGWP25GCftIOlCEAtA=";
          };
        in
        {
          packages.default = (pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          }).buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "LSP for package version management";
              homepage = "https://github.com/skanehira/version-lsp";
              license = pkgs.lib.licenses.mit;
            };
          };

          devShells.default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.cargo-nextest
              pkgs.cargo-llvm-cov
            ];

            RUST_BACKTRACE = 1;
          };
        };
    };
}
