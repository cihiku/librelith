{
  description = "";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";

    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        toolchainTomlFile = ./rust-toolchain.toml;
        toolchainToml = pkgs.lib.importTOML toolchainTomlFile;
        rustVersion = toolchainToml.toolchain.channel;

        rust = pkgs.rust-bin.stable.${rustVersion};
        rustDev = pkgs.rust-bin.fromRustupToolchainFile toolchainTomlFile;
        rustBuild = rust.minimal;
        rustCheck = rustBuild.override {
          extensions = [
            "clippy"
            "rustfmt"
          ];
        };

        craneCheck = (crane.mkLib pkgs).overrideToolchain rustCheck;

        src = self;
        workspaceToml = pkgs.lib.importTOML (src + "/Cargo.toml");
        version = workspaceToml.workspace.package.version;

      in
      rec {
        packages = { };
        checks =
          packages
          //
            removeAttrs
              (pkgs.callPackage ./nix/check.nix {
                inherit
                  formatter
                  craneCheck
                  src
                  version
                  ;
              })
              [
                "override"
                "overrideDerivation"
              ];

        devShells.default = pkgs.callPackage ./nix/devShell.nix {
          inherit
            formatter
            rustDev
            ;
        };

        formatter = pkgs.callPackage ./nix/formatter.nix {
          inherit
            rustCheck
            workspaceToml
            ;
        };

      }
    );
}
