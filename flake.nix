{
  description = "Unified job search CLI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system:
          f {pkgs = import nixpkgs {inherit system;};}
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
          # hash = pkgs.lib.fakeHash;
          hash = "sha256-opOLZqCwlCUOYG2pwJ7oZEfD2lt8fbJz/5N/rfs8f+s=";
        };

        buildPhase = ''
          pnpm build
        '';

        installPhase = ''
          cp -r dist $out
        '';
      });

      job-search = pkgs.rustPlatform.buildRustPackage {
        pname = "job-search";
        version = self.shortRev or "dirty";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = with pkgs; [pkg-config];
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
