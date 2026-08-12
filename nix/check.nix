{
  craneCheck,
  src,
  version,
  formatter,
  runCommandLocal,
  pkg-config,
  callPackage,
  cargo-hack,
}:
let
  common = {
    inherit src version;
    pname = "librelith";
    strictDeps = true;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = callPackage ./buildInputs.nix { };
  };
  cargoArtifacts = craneCheck.buildDepsOnly (common // { CARGO_PROFILE = "dev"; });
in
{
  clippy = craneCheck.cargoClippy (
    common
    // {
      inherit cargoArtifacts;
      CARGO_PROFILE = "dev";
      cargoClippyExtraArgs = "--workspace --all-targets";
    }
  );
  test = craneCheck.cargoTest (
    common
    // {
      inherit cargoArtifacts;
      CARGO_PROFILE = "dev";
      cargoTestExtraArgs = "--workspace";
    }
  );
  hack = craneCheck.mkCargoDerivation (
    common
    // {
      inherit cargoArtifacts;
      pname = "hack";
      buildPhaseCargoCommand = "cargo hack check --workspace --each-feature --no-dev-deps";
      nativeBuildInputs = common.nativeBuildInputs ++ [ cargo-hack ];
    }
  );
  fmt = runCommandLocal "treefmt-check" { nativeBuildInputs = [ formatter ]; } ''
    cp -r ${src} source
    chmod -R +w source
    cd source
    export HOME=$TMPDIR
    treefmt --ci --tree-root .
    touch $out
  '';
}
