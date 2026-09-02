{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
  packages = [pkgs.git pkgs.sqlx-cli pkgs.actionlint pkgs.shellcheck];
  env = {
    JOBSEARCH_BROWSER_BIN = "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser";
    JOBSEARCH_LLM_BIN = "pi --print --no-session --no-tools --no-extensions --mode text --thinking off --model deepseek/deepseek-v4-flash";

    SQLX_OFFLINE = "true";
    JOBSEARCH_DATABASE_URL = "sqlite:./jobsearch.db";
    JOBSEARCH_CONFIG_DIR = "./";
  };

  languages = {
    rust = {
      enable = true;
      channel = "nightly";
      components = ["rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" "rust-src"];
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
          initial_delay_seconds = 10;
          period_seconds = 5;
          failure_threshold = 10;
        };
      };
    };
  };

  scripts = {
    sqlx-update.exec = ''
      cargo sqlx database create --database-url "$JOBSEARCH_DATABASE_URL" 2>/dev/null || true
      cargo sqlx migrate run --database-url "$JOBSEARCH_DATABASE_URL" && cargo sqlx prepare --database-url "$JOBSEARCH_DATABASE_URL" -- --tests
    '';

    kill-services.exec = ''
      echo "Killing all services on ports 8080, 3000"
      pkill -9 -f "devenv" || true
      pkill -9 "process-compose" || true
      lsof -ti:8080,3000 | xargs -r kill -9 || true
    '';

    test.exec = ''
      cargo build && cargo clippy --all-targets && cargo test && cargo fmt
    '';
    test-integration.exec = ''
      cargo test -- --include-ignored
    '';
    lint-workflows.exec = ''
      actionlint
    '';
    regen-api.exec = ''
      cargo run -- serve &
      PID=$!
      until curl -s http://localhost:8080/api/openapi.json > /dev/null 2>&1; do sleep 0.5; done
      (cd frontend && pnpm orval)
      kill $PID
    '';
    export-docs.exec = ''
      RUSTDOCFLAGS="-Zunstable-options --output-format=json" cargo doc --workspace
      TARGET_DIR=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
      JSON_DIR=$(mktemp -d)
      for name in $(cargo metadata --format-version 1 | jq -r '. as $m | ([$m.resolve.nodes[] | select(.id as $id | $m.workspace_members | index($id)) | .deps[].pkg] + $m.workspace_members) | unique[] as $pid | $m.packages[] | select(.id == $pid) | .name' | sort -u); do
        json="$TARGET_DIR/doc/''${name//-/_}.json"
        [ -f "$json" ] && ln -s "$json" "$JSON_DIR/"
      done
      cargo docs-md --dir "$JSON_DIR" -o md_docs --exclude-private --source-locations --full-method-docs --hide-trivial-derives
    '';
  };
}
