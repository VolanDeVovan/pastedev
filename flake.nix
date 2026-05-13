{
  description = "pastedev — self-hostable snippet/page/HTML-share service";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Rust source filters. The CLI does not need `.sqlx/` or migrations;
        # the server needs both (sqlx offline cache + `sqlx::migrate!` macro).
        unfilteredRoot = ./.;

        cliSrc = lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = craneLib.fileset.commonCargoSources unfilteredRoot;
        };

        serverSrc = lib.fileset.toSource {
          root = unfilteredRoot;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources unfilteredRoot)
            ./.sqlx
            ./crates/server/migrations
            ./crates/server/src/http/fallback_shell.html
            ./crates/web/dist
          ];
        };

        commonArgs = {
          strictDeps = true;

          # No system libs needed: rustls everywhere, sqlx is offline.
          buildInputs = [ ];
          nativeBuildInputs = [ ];
        };

        # Workspace-wide dependency build, cached separately so editing app
        # code doesn't recompile crates.io deps.
        cargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            src = serverSrc;
            pname = "pastedev-deps";
            version = "0.1.0";

            # rust-embed's `#[folder = "../web/dist/"]` is evaluated at macro
            # expansion time. Crane stubs sources during the deps-only build,
            # but a missing directory still trips the macro — placeholder it.
            preBuild = ''
              mkdir -p crates/web/dist
            '';
            SQLX_OFFLINE = "true";
          }
        );

        pastedev-cli = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            src = cliSrc;
            pname = "pastedev-cli";
            version = "0.1.0";
            cargoExtraArgs = "--locked --package pastedev-cli";
            doCheck = false;

            meta = {
              description = "Terminal client + MCP server for pastedev";
              homepage = "https://github.com/volandevovan/pastedev";
              license = lib.licenses.mit;
              mainProgram = "pastedev-cli";
            };
          }
        );

        # The Dioxus SPA must be built outside Nix (`just build-web`) before
        # invoking `nix build .#pastedev-server`: dx fetches its toolchain at
        # runtime, which doesn't work in the sandbox. The serverSrc fileset
        # above includes `./crates/web/dist`, so as long as it's populated
        # the rust-embed macro picks it up. Production builds go through the
        # Dockerfile, which builds the SPA inside stage 1 of the image.
        pastedev-server = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            src = serverSrc;
            pname = "pastedev-server";
            version = "0.1.0";
            cargoExtraArgs = "--locked --package pastedev-server";
            doCheck = false;
            SQLX_OFFLINE = "true";

            meta = {
              description = "pastedev HTTP server with embedded SPA";
              homepage = "https://github.com/volandevovan/pastedev";
              license = lib.licenses.mit;
              mainProgram = "pastedev-server";
            };
          }
        );
      in
      {
        packages = {
          inherit pastedev-cli pastedev-server;
          default = pastedev-cli;
        };

        apps = {
          pastedev-cli = flake-utils.lib.mkApp { drv = pastedev-cli; };
          pastedev-server = flake-utils.lib.mkApp { drv = pastedev-server; };
          default = flake-utils.lib.mkApp { drv = pastedev-cli; };
        };
      }
    );
}
