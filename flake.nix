{
  description = "Aria development flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};
    rustupToolchain = "stable";
    rustBuildTargetTriple = "x86_64-pc-windows-gnu";

    # Cross-compilation package set for Windows target
    pkgsCross = import nixpkgs {
      inherit system;
      crossSystem = {config = "x86_64-w64-mingw32";};
    };

    mingw_w64_cc = pkgsCross.stdenv.cc;
    mingw_w64 = pkgsCross.windows.mingw_w64;
    mingw_w64_pthreads_w_static = pkgsCross.windows.mingw_w64_pthreads.overrideAttrs (oldAttrs: {
      configureFlags = (oldAttrs.configureFlags or []) ++ ["--enable-static"];
    });
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [rustup mingw_w64_cc yq cmake];

      # Default Rust toolchain and target
      RUSTUP_TOOLCHAIN = rustupToolchain;
      CARGO_BUILD_TARGET = rustBuildTargetTriple;

      shellHook = ''
        # Ensure our Windows target is added and rustfmt is available
        rustup target add "${rustBuildTargetTriple}"
        rustup component add rustfmt

        # Use Windows system name via toolchain file
        export CMAKE_TOOLCHAIN_FILE="$PWD/mingw-generic.cmake"
        # Set cross compilers
        export CC=x86_64-w64-mingw32-gcc
        export CXX=x86_64-w64-mingw32-g++
        export RC=x86_64-w64-mingw32-windres

        # Clear default and initial linker flags to remove --major-image-version
        export CMAKE_EXE_LINKER_FLAGS_INIT=""
        export CMAKE_SHARED_LINKER_FLAGS_INIT=""
        export CMAKE_MODULE_LINKER_FLAGS_INIT=""
        export CMAKE_EXE_LINKER_FLAGS=""
        export CMAKE_SHARED_LINKER_FLAGS=""
        export CMAKE_MODULE_LINKER_FLAGS=""
      '';

      # Pass linker flags for static pthread support
      RUSTFLAGS = builtins.map (a: ''-L ${a}/lib'') [mingw_w64 mingw_w64_pthreads_w_static];
    };

    packages.${system}.default = pkgsCross.rustPlatform.buildRustPackage {
      pname = "aria";
      name = "aria";
      src = ./.;
      cargoLock.lockFile = ./Cargo.lock;
      nativeBuildInputs = [mingw_w64_cc pkgs.cmake];
      buildInputs = [mingw_w64 mingw_w64_pthreads_w_static];
    };
  };
}
