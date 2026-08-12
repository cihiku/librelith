{
  mkShell,
  rustDev,
  tombi,
  just,
  pkg-config,
  clang,
  formatter,
}:
mkShell {
  packages = [
    rustDev
    pkg-config
    clang
    tombi
    just
    formatter
  ];
}
