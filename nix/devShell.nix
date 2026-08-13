{
  mkShell,
  rustDev,
  tombi,
  just,
  pkg-config,
  clang,
  formatter,
  cocogitto,
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
  ];
}
