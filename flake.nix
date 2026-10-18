{
  description = "Lemmata: development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
    in
    {
      devShells = forAllSystems (pkgs: {
        # The browser suite in §7.1 needs a real browser and a driver. They are
        # not in the default shell because the default shell has to build from the
        # store alone, and these are large. `pnpm install` inside harness/browser
        # needs the network too.
        browser = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_22
            pnpm
            playwright-driver
            playwright-driver.browsers
          ];
        };

        default = pkgs.mkShell {
          packages = with pkgs; [
            # The toolchain is the one rust-toolchain.toml pins, components and
            # targets included, so the shell and a rustup checkout agree.
            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)

            # The cargo config in ~/.cargo wraps rustc in sccache and links with
            # clang, so host builds fail without both.
            sccache
            clang
            pkg-config

            # `cargo xtask parity` builds the wasm side with this, and the
            # oracle and web tests need node.
            nodejs_22
            wasm-pack
            pnpm

            # The validity gates the plan asks for. `ots-sanitize` is
            # r[build.valid]'s other half, and taplo owns the TOML formatting.
            opentype-sanitizer
            taplo
            cargo-deny
            ast-grep

            # r[math.typst] compiles the equation corpus with this.
            typst

            # The browser build. The CLI is pinned to match wasm-bindgen in
            # [workspace.dependencies], so a version bump has to move both.
            wasm-bindgen-cli
            binaryen

            # fontconfig so a generated font can be inspected the way a
            # renderer sees it, and python3 for one-off probes.
            fontconfig
            python3
            git
          ];

          shellHook = ''
            echo "lemmata: $(rustc --version)"
            echo "         ots-sanitize $(ots-sanitize --version 2>/dev/null | head -1)"
          '';
        };
      });
    };
}
