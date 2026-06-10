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
      exec = "pnpm --dir frontend start";
      process-compose = {
        depends_on.backend.condition = "process_healthy";
      };
    };
    backend = {
      exec = "cargo run -- serve";
      process-compose = {
        readiness_probe = {
          http_get = {
            host = "127.0.0.1";
            port = 8080;
            path = "/health";
          };
          initial_delay_seconds = 5;
          period_seconds = 2;
        };
      };
    };
  };
  scripts = {
    test.exec = ''
      cargo build && cargo clippy -- -D warnings && cargo test && cargo fmt
    '';
    test-integration.exec = ''
      cargo test -- --include-ignored
    '';
    regen-api.exec = ''
      cargo run -- serve &
      PID=$!
      until curl -s http://localhost:8080/api/openapi.json > /dev/null 2>&1; do sleep 0.5; done
      pnpm --dir frontend orval
      kill $PID
    '';
    export-docs.exec = ''
      RUSTDOCFLAGS="-Zunstable-options --output-format=json" cargo doc
      cargo docs-md --dir target/doc/ -o target/md_docs  --source-locations --full-method-docs --hide-trivial-derives
    '';
  };
}
