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

        # SPA bundle (legacy Vue). Still produced for the default-feature
        # server build path; `pastedev-web-dioxus` below replaces it when the
        # server is built with the `dioxus-spa` feature.
        # The `pnpmDeps.hash` below pins the resolved pnpm store. When
        # `web/pnpm-lock.yaml` changes, replace with `lib.fakeHash` once,
        # run `nix build .#pastedev-web`, and copy the SRI hash Nix prints.
        pastedev-web = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "pastedev-web";
          version = "0.1.0";

          src = lib.cleanSource ./web;

          pnpmDeps = pkgs.fetchPnpmDeps {
            inherit (finalAttrs) pname version src;
            fetcherVersion = 3;
            hash = "sha256-zQbW9OBN2p46A3KP8crdRH61nkNhj8JM8DNY2Kvf+zE=";
          };

          nativeBuildInputs = [
            pkgs.nodejs_22
            pkgs.pnpm_10
            pkgs.pnpmConfigHook
          ];

          buildPhase = ''
            runHook preBuild
            pnpm run build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -r dist $out
            runHook postInstall
          '';
        });

        # The Dioxus SPA (`crates/web/`) builds outside Nix because dx fetches
        # its own toolchain at runtime, which doesn't work in the sandbox.
        # `flake.nix` keeps producing the legacy Vue bundle for the default
        # server build path; phase 4 of the migration deletes both this Vue
        # derivation and the legacy `web/` tree. Until then, anyone building
        # the Dioxus-flavor server image should go through the Dockerfile
        # (`SPA_FLAVOR=dioxus`, the default) instead of `nix build`.

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

            # rust-embed's `#[folder = "../../web/dist/"]` is evaluated at
            # macro-expansion time. Crane stubs sources during the deps-only
            # build, but a missing directory still trips the macro — give it
            # an empty placeholder.
            preBuild = ''
              mkdir -p web/dist
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

        pastedev-server = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            src = serverSrc;
            pname = "pastedev-server";
            version = "0.1.0";
            cargoExtraArgs = "--locked --package pastedev-server";
            doCheck = false;

            preBuild = ''
              mkdir -p web
              cp -r ${pastedev-web} web/dist
            '';
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
          inherit pastedev-cli pastedev-server pastedev-web;
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
