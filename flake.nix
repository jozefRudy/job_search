{
  description = "Unified job search CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
  }: let
    supportedSystems = ["aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system: let
          pkgs = import nixpkgs {inherit system;};
          craneLib = crane.mkLib pkgs;
        in
          f {inherit pkgs craneLib;}
      );
  in {
    packages = forEachSupportedSystem ({
      pkgs,
      craneLib,
    }: let
      pnpm = pkgs.pnpm_10;

      frontendVersion = (builtins.fromJSON (builtins.readFile ./frontend/package.json)).version;
      cargoVersion = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;

      frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
        pname = "jobsearch-frontend";
        version = frontendVersion;
        src = ./frontend;

        nativeBuildInputs = [
          pkgs.nodejs-slim
          pnpm
          pkgs.pnpmConfigHook
        ];

        pnpmDeps = pkgs.fetchPnpmDeps {
          inherit (finalAttrs) pname version src;
          inherit pnpm;
          fetcherVersion = 3;
          # When pnpm dependencies change, swap to pkgs.lib.fakeHash, run `nix build .#frontend`,
          # copy the "got:" hash from the error, then put it back here.
          hash = "sha256-5Dc5RFYuoVDs6uOon260I/016CvFCfln8zGvkeBzVmo";
        };

        buildPhase = ''
          pnpm build
        '';

        installPhase = ''
          cp -r dist $out
        '';
      });

      onnxruntime-bin = let
        version = "1.24.2";
        src = pkgs.fetchurl {
          url = "https://github.com/microsoft/onnxruntime/releases/download/v${version}/onnxruntime-osx-arm64-${version}.tgz";
          hash = "sha256-CvT6UD6OooUkW0fuQtCnRhuBVqgScIV9oMHU7PhYq94=";
        };
      in
        pkgs.stdenvNoCC.mkDerivation {
          inherit src version;
          pname = "onnxruntime-bin";
          dontBuild = true;
          installPhase = ''
            mkdir -p $out
            cp -r lib $out/lib
            cp -r include $out/include
          '';
        };

      srcForPackage = pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: type: let
          base = baseNameOf path;
        in
          !(
            base
            == ".git"
            || base == ".devenv"
            || base == ".direnv"
            || base == "frontend"
            || base == "models"
            || base == "lance"
            || base == "target"
            || base == "jobsearch.db"
            || base == "providers.md"
          );
      };

      srcForDeps = craneLib.cleanCargoSource (craneLib.path ./.);

      commonArgs = {
        pname = "job-search";
        version = cargoVersion;
        src = srcForDeps;
        nativeBuildInputs = [pkgs.pkg-config pkgs.protobuf];
        buildInputs = [onnxruntime-bin];
        env = {
          SQLX_OFFLINE = "true";
          ORT_PREFER_DYNAMIC_LINK = "1";
          ORT_LIB_PATH = "${onnxruntime-bin}/lib";
          RUSTFLAGS = "-Clink-arg=-Wl,-rpath,${onnxruntime-bin}/lib";
        };
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      job-search = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
          src = srcForPackage;
          version = self.shortRev or "dirty";
          env =
            commonArgs.env
            // {
              GIT_HASH = self.shortRev or "dirty";
            };
          preBuild = ''
            mkdir -p frontend/dist
            cp -r ${frontend}/* frontend/dist/
          '';
        });
    in {
      inherit frontend job-search;
      default = job-search;
    });
  };
}
