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
    packages = forEachSupportedSystem (
      {pkgs}: {
        job-search = pkgs.rustPlatform.buildRustPackage {
          pname = "job-search";
          version = self.shortRev or "dirty";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [pkg-config];
        };
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.job-search;
      }
    );
  };
}
