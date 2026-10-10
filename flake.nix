{
  description = "CLI and desktop tools for managing projects and Git worktrees";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeCheckInputs = [ pkgs.git pkgs.bash pkgs.zsh ];
          # libiconv is required by Rust crates on Darwin
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };

        # Build dependencies in a separate derivation so they can be cached
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        spacer = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
          postInstall = ''
            wrapProgram "$out/bin/spacer" --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.git ]}
          '';
          meta.mainProgram = "spacer";
        });
        desktop = pkgs.callPackage ./nix/desktop.nix { };
      in
      {
        packages = {
          default = spacer;
          cli = spacer;
          inherit desktop;
        };

        checks = {
          inherit spacer;
          inherit desktop;
          desktop-smoke = desktop.override { nativeSmoke = true; };

          spacer-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
          });

          spacer-fmt = craneLib.cargoFmt { inherit src; };

          spacer-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
            cargoTestExtraArgs = "--workspace";
          });
        };

        devShells.default = craneLib.devShell {
          checks = builtins.removeAttrs self.checks.${system} [ "desktop" "desktop-smoke" ];
        };
      }
    );
}
