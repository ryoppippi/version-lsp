{
  description = "version-lsp - LSP for package version management";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, crane, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { system, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchainToml = pkgs.lib.importTOML ./rust-toolchain.toml;
          rustVersion = rustToolchainToml.toolchain.channel;

          # Minimal toolchain (for build)
          rustToolchainMinimal = pkgs.rust-bin.stable.${rustVersion}.minimal;
          craneLibMinimal = (crane.mkLib pkgs).overrideToolchain rustToolchainMinimal;

          # Full toolchain from rust-toolchain.toml (for dev)
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          commonArgs = {
            src = (crane.mkLib pkgs).cleanCargoSource ./.;
          };

          cargoArtifacts = craneLibMinimal.buildDepsOnly commonArgs;

          meta = {
            description = "LSP for package version management";
            homepage = "https://github.com/skanehira/version-lsp";
            license = pkgs.lib.licenses.mit;
          };
        in
        {
          packages = {
            # 成果物ビルド。テストは `nix flake check` (checks.test) で実行するため
            # ここでは doCheck = false とし、ビルドを hermetic かつ高速に保つ。
            default = craneLibMinimal.buildPackage (commonArgs // {
              inherit cargoArtifacts meta;
              doCheck = false;
            });

            # CI build (skip tests)
            ci = craneLibMinimal.buildPackage (commonArgs // {
              inherit cargoArtifacts meta;
              doCheck = false;
            });
          };

          # `nix flake check` でテストを実行する。
          # reqwest (rustls) がクライアント生成時にシステムの CA 証明書を要求するため
          # SSL_CERT_FILE を渡す。テスト自体は mockito (ローカルモック) ベースで
          # 外部ネットワークには出ないため、これで sandbox 内でも完結する。
          checks = {
            test = craneLibMinimal.cargoTest (commonArgs // {
              inherit cargoArtifacts;
              SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            });
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
