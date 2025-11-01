{
    description = "Osiris devshell flake";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        rust-overlay.url = "github:oxalica/rust-overlay";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs =
        {
            self,
            nixpkgs,
            rust-overlay,
            flake-utils,
            ...
        }:
        flake-utils.lib.eachDefaultSystem (
            system:
            let
                overlays = [ (import rust-overlay) ];
                pkgs = import nixpkgs {
                    inherit system overlays;
                };
            in
            {
                devShells.default = pkgs.mkShell {
                    buildInputs = with pkgs; [
                        just
                        gcc-arm-embedded
                        python313
                        python313Packages.cmake
                        python313Packages.pyelftools
                        (rust-bin.stable.latest.default.override {
                            extensions = [
                                "cargo"
                                "llvm-tools"
                                "rust-analysis"
                                "rust-analyzer"
                                "rust-docs"
                                "rustfmt"
                            ];
                            targets = [
                                "thumbv7em-none-eabi"
                            ];
                        })
                        cargo-binutils
                        llvm
                        clang
                        glibc_multi
                        ninja

                        stlink
                    ];

                    shellHook = ''
                        export LIBCLANG_PATH=${pkgs.llvmPackages.libclang.lib}/lib
                    '';
                };
            }
        );
}
