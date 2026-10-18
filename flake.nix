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

            # `cargo xtask` needs neither, but the oracle and web tests do.
            nodejs_22

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
