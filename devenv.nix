{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  packages = [pkgs.git];

  languages = {
    rust = {
      enable = true;
      channel = "nightly";
    };
  };
  scripts = {
    validate.exec = ''
      cargo build && cargo clippy -- -D warnings && cargo test && cargo fmt
    '';
    validate-integration.exec = ''
      cargo test -- --include-ignored
    '';
    export-docs.exec = ''
      RUSTDOCFLAGS="-Zunstable-options --output-format=json" cargo doc
      cargo docs-md --dir target/doc/ -o target/md_docs  --source-locations --full-method-docs --hide-trivial-derives
    '';
  };
}
