{
  description = "Flow Nexus v1 schema-only acceptance";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-build.url = "github:LiGoldragon/rust-build/1bcdafd4590952f73fada56b0507c64192fd6327";
    rust-build.inputs.nixpkgs.follows = "nixpkgs";
    flow = { url = "github:LiGoldragon/flow-nexus/cd4a184cf571a26ab539d1a6ac633d0243d97526"; flake = false; };
    ordinary = { url = "github:LiGoldragon/signal-flow-nexus/11734f0502586378c2b055e51e81b8c4f9e6d1fd"; flake = false; };
    memory = { url = "github:LiGoldragon/memory/bfdde49897d060ce29f962ee35bdfaed3b50dd6b"; flake = false; };
    meta = { url = "github:LiGoldragon/meta-signal-flow-nexus/7d47476a61e3a5ec7b9b813a910a6c40798ba4bf"; flake = false; };
  };
  outputs = { self, nixpkgs, rust-build, flow, ordinary, meta, memory }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      rust = rust-build.lib.${system}.fromPkgs pkgs;
      src = pkgs.runCommand "flow-nexus-schema-test-source" {} ''
        mkdir -p $out/memory $out/flow-nexus $out/signal-flow-nexus $out/meta-signal-flow-nexus $out/tests
        cp -r ${flow}/* $out/flow-nexus/
        cp -r ${ordinary}/* $out/signal-flow-nexus/
        cp -r ${meta}/* $out/meta-signal-flow-nexus/
        cp -r ${memory}/* $out/memory/
        cp ${./Cargo.toml} $out/Cargo.toml
        cp ${./Cargo.lock} $out/Cargo.lock
        cp -r ${./tests}/* $out/tests/
      '';
    in {
      checks.${system} = {
      normal-closure = rust.craneLib.cargoBuild {
        inherit src;
        pname = "flow-nexus-normal-closure";
        version = "0.1.0";
        cargoVendorDir = rust.craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
        cargoArtifacts = null;
        doInstallCargoArtifacts = false;
        cargoExtraArgs = "--locked --no-default-features";
        postBuild = ''
          cargo tree --offline --locked --no-default-features --edges normal --prefix none > normal-dependencies.txt
          if grep -E '^(datom-codec|protos) ' normal-dependencies.txt; then
            echo "text dependency in binary-only normal dependency graph" >&2
            exit 1
          fi
        '';
        strictDeps = true;
      };
      schema = rust.craneLib.cargoTest {
        inherit src;
        pname = "flow-nexus-schema-tests";
        version = "0.1.0";
        cargoVendorDir = rust.craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
        cargoArtifacts = null;
        doInstallCargoArtifacts = false;
        cargoExtraArgs = "--locked --all-features";
        strictDeps = true;
        postUnpack = ''sourceRoot="$sourceRoot"'';
      };
      newtype-projection = rust.craneLib.cargoTest {
        inherit src;
        pname = "flow-nexus-newtype-projection";
        version = "0.1.0";
        cargoVendorDir = rust.craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
        cargoArtifacts = null;
        doInstallCargoArtifacts = false;
        cargoExtraArgs = "--locked --all-features --test newtype_projection -- --nocapture";
        strictDeps = true;
        postUnpack = ''sourceRoot="$sourceRoot"'';
      };

      memory = rust.craneLib.cargoTest {
        inherit src;
        pname = "flow-nexus-memory-tests";
        version = "0.1.0";
        cargoVendorDir = rust.craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
        cargoArtifacts = null;
        doInstallCargoArtifacts = false;
        cargoExtraArgs = "--locked --test memory -- --nocapture";
        strictDeps = true;
        postUnpack = ''sourceRoot="$sourceRoot"'';
      };
      };
      devShells.${system}.default = pkgs.mkShell { packages = [ rust.toolchain ]; };
    };
}
