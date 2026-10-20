{
  description = "Lemmata: a programmatic typeface factory";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # The upstream tree the definitions mirror. A flake source leaves submodules
    # out, so the parameter files the glyph source reads at compile time come
    # from here rather than from `vendor/Iosevka`.
    iosevka = {
      url = "github:be5invis/Iosevka/2487637bb71f41b2a66134afd9faa6c316e5a438";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      iosevka,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          lib = pkgs.lib;

          # The toolchain `rust-toolchain.toml` pins, components and targets
          # included, so a Nix build and a rustup checkout agree.
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          lemmata = rustPlatform.buildRustPackage {
            pname = "lemmata";
            version = "0.1.0";
            # The git tree, so `vendor/kurbo` (a tracked patch) is present and
            # the `vendor/Iosevka` submodule, which is not, is not expected.
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [
              "-p"
              "lemmata-cli"
            ];

            # `lemmata-web` reads the parameter files at compile time, and the
            # submodule that carries them is not part of the source.
            preBuild = ''
              mkdir -p vendor/Iosevka
              cp -r ${iosevka}/params vendor/Iosevka/params
            '';

            # The suite wants a capture, a reference build and a browser. Those
            # are the oracle's inputs, not the package's.
            doCheck = false;

            installPhase = ''
              runHook preInstall
              install -Dm755 "target/${pkgs.stdenv.hostPlatform.rust.cargoShortTarget}/release/lemmata" $out/bin/lemmata
              runHook postInstall
            '';

            meta = {
              description = "Build a Lemmata font from a plan and a glyph capture";
              license = lib.licenses.mit;
              mainProgram = "lemmata";
            };
          };

          # The committed capture is the one the site ships, so the package
          # builds the same font the page does.
          capture = ./web/public/glyphs.bin;

          # `mkFont` takes a plan the way the Iosevka package does: an attrset,
          # or the JSON a file already holds. The plan reaches the font's
          # metrics and its variant choices; the outlines are the capture's
          # shape, so a plan that asks for another weight or slope needs a
          # capture of that shape.
          mkFont =
            {
              family ? "Lemmata",
              name ? "Lemmata-Regular",
              extension ? "ttf",
              capture ? ./web/public/glyphs.bin,
              plan ? { },
            }:
            pkgs.runCommand "lemmata-${name}-${extension}" {
              nativeBuildInputs = [ lemmata ];
              meta.description = "A Lemmata font, ${name}.${extension}";
            } ''
              kind=${if extension == "woff2" then "woff2" else "truetype"}
              mkdir -p "$out/share/fonts/$kind"
              lemmata build \
                --glyphs ${capture} \
                --family ${lib.escapeShellArg family} \
                --plan ${
                  if builtins.isString plan then
                    pkgs.writeText "lemmata-plan.json" plan
                  else
                    pkgs.writeText "lemmata-plan.json" (builtins.toJSON plan)
                } \
                --out "$out/share/fonts/$kind/${name}.${extension}"
            '';

          fonts = pkgs.buildEnv {
            name = "lemmata-fonts";
            paths = [
              (mkFont { extension = "ttf"; })
              (mkFont { extension = "woff2"; })
            ];
          };
        in
        {
          packages = {
            inherit lemmata fonts;
            default = fonts;
          };

          lib = { inherit mkFont; };

          apps.default = {
            type = "app";
            program = "${lemmata}/bin/lemmata";
          };

          devShells = {
            # The browser suite needs a real browser and a driver. They are not
            # in the default shell because the default shell has to build from
            # the store alone, and these are large. `pnpm install` inside
            # harness/browser needs the network too.
            browser = pkgs.mkShell {
              packages = with pkgs; [
                nodejs_22
                pnpm
                playwright-driver
                playwright-driver.browsers
              ];

              # Playwright looks in ~/.cache/ms-playwright by default, so it has
              # to be told where the store put the engines.
              PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
            };

            default = pkgs.mkShell {
              packages = with pkgs; [
                (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)

                # The cargo config wraps rustc in sccache and links with clang,
                # so host builds fail without both.
                sccache
                clang
                pkg-config

                # `cargo xtask parity` builds the wasm side with this, and the
                # oracle and web tests need node.
                nodejs_22
                wasm-pack
                pnpm

                # The validity gates the plan asks for.
                opentype-sanitizer
                taplo
                cargo-deny
                ast-grep

                # The equation corpus.
                typst

                # The browser build, pinned to match wasm-bindgen.
                wasm-bindgen-cli
                binaryen

                fontconfig
                python3
                git
              ];

              shellHook = ''
                echo "lemmata: $(rustc --version)"
                echo "         ots-sanitize $(ots-sanitize --version 2>/dev/null | head -1)"
              '';
            };
          };
        };
    in
    {
      packages = nixpkgs.lib.genAttrs systems (system: (perSystem system).packages);
      apps = nixpkgs.lib.genAttrs systems (system: (perSystem system).apps);
      devShells = nixpkgs.lib.genAttrs systems (system: (perSystem system).devShells);
      lib = nixpkgs.lib.genAttrs systems (system: (perSystem system).lib);
    };
}
