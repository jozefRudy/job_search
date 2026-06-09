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
    javascript = {
      enable = true;
      package = pkgs.nodejs_24;
      pnpm.enable = true;
      pnpm.install.enable = true;
      directory = "./frontend";
    };
    typescript = {
      enable = true;
      lsp.enable = true;
    };
  };

  processes = {
    frontend = {
      exec = "(cd frontend && pnpm start)";
    };
    backend = {
      exec = "cargo run -- serve";
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
