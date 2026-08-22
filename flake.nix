{
  description = "Unified job search CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    # hashes are updated by .github/workflows/release.yml after each release
    version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
    hashes = {
      aarch64-darwin = "sha256-s88F5/UVHDHCCHHFMKsOEIP0Y7z9t+5tyK2XTQgDt+Q=";
    };
    assets = {
      aarch64-darwin = "aarch64-macos";
    };

    supportedSystems = builtins.attrNames assets;

    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system:
          f {
            inherit system;
            pkgs = import nixpkgs {inherit system;};
          }
      );
  in {
    packages = forEachSupportedSystem ({
      system,
      pkgs,
    }: let
      inherit (pkgs) lib;
      job-search = pkgs.stdenv.mkDerivation {
        pname = "job-search";
        inherit version;

        src = pkgs.fetchurl {
          url = "https://github.com/jozefRudy/job_search/releases/download/v${version}/jobsearch-v${version}-${assets.${system}}.tar.gz";
          hash = hashes.${system};
        };

        dontBuild = true;
        dontConfigure = true;
        dontStrip = true;

        nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [pkgs.patchelf];

        installPhase = ''
          mkdir -p $out
          tar -xzf $src -C $out --strip-components=1
        '';

        postFixup = lib.optionalString pkgs.stdenv.isLinux ''
          patchelf \
            --force-rpath \
            --set-interpreter "${pkgs.stdenv.cc.bintools.dynamicLinker}" \
            --set-rpath "${lib.makeLibraryPath [pkgs.stdenv.cc.cc.lib pkgs.openssl]}:$out/lib" \
            $out/bin/jobsearch
        '';

        meta = {
          description = "Unified job search CLI";
          homepage = "https://github.com/jozefRudy/job_search";
          mainProgram = "jobsearch";
          platforms = supportedSystems;
        };
      };
    in {
      inherit job-search;
      default = job-search;
    });
  };
}
