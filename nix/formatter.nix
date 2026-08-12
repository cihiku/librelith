{
  nixfmt-tree,
  tombi,
  rustCheck,
  workspaceToml,
}:
nixfmt-tree.override {
  runtimeInputs = [
    tombi
    rustCheck
  ];
  settings = {
    formatter = {
      tombi = {
        command = "tombi";
        options = [
          "format"
          "--offline"
        ];
        includes = [ "*.toml" ];
      };
      rustfmt = {
        command = "rustfmt";
        options = [
          "--edition"
          workspaceToml.workspace.package.edition
        ];
        includes = [ "*.rs" ];
      };
    };
  };
}
