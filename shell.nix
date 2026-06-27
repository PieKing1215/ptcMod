# Based on https://nixos.wiki/wiki/Rust#Installation_via_rustup

{ pkgs ? import <nixpkgs> {} }:
  let
    overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
    libPath = with pkgs; lib.makeLibraryPath [
      # load external libraries that you need in your rust project here
    ];
in
  pkgs.mkShell rec {
    buildInputs = with pkgs; [
      clang
      llvmPackages_19.bintools
      rustup
      cargo-xwin
      cargo-cross
      pkg-config
      pkgsCross.mingw32.stdenv.cc
      pkgsCross.mingw32.buildPackages.gcc
      pkgsCross.mingw32.windows.pthreads
    ];
  }
