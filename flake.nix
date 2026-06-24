{
  description = "Rust";
  inputs = {
    # nix stuff
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      treefmt-nix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";

          # Enable the formatters you need
          programs.nixpkgs-fmt.enable = true; # Formats .nix files
          programs.rustfmt.enable = true; # Formats .rs files via cargo
        };

        # for non-nix users https://hgdownload.gi.ucsc.edu/downloads.html#utilities_downloads
        liftOver = pkgs.stdenv.mkDerivation rec {
          pname = "liftOver";
          version = "latest";

          # hashes may require adjustment
          src =
            if pkgs.stdenv.isDarwin then
              pkgs.fetchurl {
                url = "https://hgdownload.soe.ucsc.edu/admin/exe/macOSX.x86_64/liftOver";
                sha256 = "sha256-9lg7+MXpUrMsZK9tAkpHAQWJPS16RDrByno1iC/8kuA=";
              }
            else
              pkgs.fetchurl {
                url = "https://hgdownload.soe.ucsc.edu/admin/exe/linux.x86_64/liftOver";
                sha256 = "sha256-0000000000000000000000000000000000000000000="; # Placeholder
              };

          # Since we are fetching a bare binary, we need to skip the unpack phase
          dontUnpack = true;

          # For Linux, we often need to patch the binary to find the right libraries
          nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.autoPatchelfHook ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.zlib
            pkgs.openssl
            pkgs.libpng
          ];

          installPhase = ''
            mkdir -p $out/bin
            cp $src $out/bin/liftOver
            chmod +x $out/bin/liftOver
          '';
        };
      in
      {
        devShells.default = pkgs.mkShellNoCC {
          packages = with pkgs; [
            cargo
            cmake
            # bio
            htslib # bgzip
            bedtools
            liftOver
            # misc
            just
            libiconv
            # analysis
            uv
            samply
          ];

          # Help the linker find libiconv on Darwin
          LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isDarwin "${pkgs.libiconv}/lib";
        };

        formatter = treefmtEval.config.build.wrapper;
      }
    );
}
