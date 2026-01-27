{
  description = "Git Worktree Manager - A TUI application for managing git worktrees";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        # Filter source to only include Rust-related files
        src = craneLib.cleanCargoSource ./.;

        # Common arguments for crane builds
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = [ pkgs.pkg-config ];

          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.apple-sdk_15
          ];
        };

        # Build dependencies separately for better caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual package
        gwm = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;

          # Tests are run separately in checks.gwm-nextest
          doCheck = false;
        });
      in
      {
        packages = {
          default = gwm;
          gwm = gwm;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = gwm;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from the main package
          inputsFrom = [ gwm ];

          # Additional development tools
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
            cargo-edit
          ];
        };

        checks = {
          # Build the package as a check
          inherit gwm;

          # Run clippy
          gwm-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          # Check formatting
          gwm-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Run tests
          gwm-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";

            # Git is required for tests
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.git ];

            # Tests require git configuration
            preCheck = ''
              export HOME=$(mktemp -d)
              git config --global user.email "test@example.com"
              git config --global user.name "Test User"
              git config --global init.defaultBranch main
            '';
          });
        };
      }
    ) // {
      # Overlay for use in other flakes
      overlays.default = final: prev: {
        gwm = self.packages.${final.system}.default;
      };
    };
}
