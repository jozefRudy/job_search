{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  packages = [pkgs.git pkgs.sqlx-cli];
  env = {
    SQLX_OFFLINE = "true";
    DATABASE_URL = "sqlite:./jobsearch.db";
  };
  enterShell = ''
    cargo sqlx database create 2>/dev/null || true
    cargo sqlx migrate run && cargo sqlx prepare
  '';

  languages = {
    rust = {
      enable = true;
      channel = "nightly";
    };
  };
  scripts = {
    test.exec = ''
      cargo build && cargo clippy -- -D warnings && cargo test && cargo fmt
    '';
    test-integration.exec = ''
      cargo test -- --include-ignored
    '';
    export-docs.exec = ''
      RUSTDOCFLAGS="-Zunstable-options --output-format=json" cargo doc
      cargo docs-md --dir target/doc/ -o target/md_docs  --source-locations --full-method-docs --hide-trivial-derives
    '';
  };
}
