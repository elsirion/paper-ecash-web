{
  description = "Paper eCash Web – Leptos + Fedimint WASM app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            binaryen
            clang
            cmake
            openssl
            perl
            pkg-config
            protobuf
            rust-analyzer
            rustToolchain
            trunk
            wasm-bindgen-cli
          ];

          env = {
            RUST_BACKTRACE = "1";
          };

          shellHook = ''
            export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

            CLANG_UNWRAPPED="$(which -a clang | sed -n '2p')"
            CLANGXX_UNWRAPPED="$(which -a clang++ | sed -n '2p')"

            if [ -z "$CLANG_UNWRAPPED" ]; then
              CLANG_UNWRAPPED="$(command -v clang)"
            fi
            if [ -z "$CLANGXX_UNWRAPPED" ]; then
              CLANGXX_UNWRAPPED="$(command -v clang++)"
            fi

            export CC_wasm32_unknown_unknown="$CLANG_UNWRAPPED"
            export CXX_wasm32_unknown_unknown="$CLANGXX_UNWRAPPED"
            export AR_wasm32_unknown_unknown="ar"

            cat <<'EOF'
            Paper eCash Web dev shell ready.

            Useful commands:
              trunk serve
              trunk build
            EOF
          '';
        };
      });
}
