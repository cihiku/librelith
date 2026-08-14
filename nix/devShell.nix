{
  mkShell,
  rustDev,
  tombi,
  just,
  pkg-config,
  clang,
  formatter,
  cocogitto,
  cargo-edit,
}:
mkShell {
  packages = [
    rustDev
    pkg-config
    clang
    tombi
    just
    formatter
    cocogitto
    cargo-edit
  ];
}
