{
  description = "Flow Nexus v1 schema-only acceptance";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-build.url = "github:LiGoldragon/rust-build/1bcdafd4590952f73fada56b0507c64192fd6327";
    rust-build.inputs.nixpkgs.follows = "nixpkgs";
    flow = { url = "github:LiGoldragon/flow-nexus/db7423cdbd38b8d2834b793416e6b6141f8594b8"; flake = false; };
    ordinary = { url = "github:LiGoldragon/signal-flow-nexus/0bc74491f682ec5d0f037c64611f9fa30a574346"; flake = false; };
    meta = { url = "github:LiGoldragon/meta-signal-flow-nexus/145eaa3d2fc1a6796549c68dbb7d3fadb80c026e"; flake = false; };
  };
  outputs = { self, nixpkgs, rust-build, flow, ordinary, meta }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      rust = rust-build.lib.${system}.fromPkgs pkgs;
      src = pkgs.runCommand "flow-nexus-schema-test-source" {} ''
        mkdir -p $out/flow-nexus $out/signal-flow-nexus $out/meta-signal-flow-nexus $out/tests
        cp -r ${flow}/* $out/flow-nexus/
        cp -r ${ordinary}/* $out/signal-flow-nexus/
        cp -r ${meta}/* $out/meta-signal-flow-nexus/
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
      };
      devShells.${system}.default = pkgs.mkShell { packages = [ rust.toolchain ]; };
    };
}
