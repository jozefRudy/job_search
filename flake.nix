{
  description = "Unified job search CLI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = ["aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system: let
          pkgs = import nixpkgs {inherit system;};
        in
          f {inherit pkgs;}
      );
  in {
    packages = forEachSupportedSystem ({pkgs}: let
      pnpm = pkgs.pnpm_10;

      frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
        pname = "jobsearch-frontend";
        version = self.shortRev or "dirty";
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

      job-search = pkgs.rustPlatform.buildRustPackage {
        pname = "job-search";
        version = self.shortRev or "dirty";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = with pkgs; [pkg-config protobuf];
        buildInputs = [onnxruntime-bin];
        env = {
          GIT_HASH = self.shortRev or "dirty";
          SQLX_OFFLINE = "true";
          ORT_PREFER_DYNAMIC_LINK = "1";
          ORT_LIB_PATH = "${onnxruntime-bin}/lib";
        };
        preBuild = ''
          mkdir -p frontend/dist
          cp -r ${frontend}/* frontend/dist/
        '';
      };
    in {
      inherit frontend job-search;
      default = job-search;
    });
  };
}
