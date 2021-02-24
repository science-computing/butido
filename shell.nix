{ example ? "1", ... }:

let
  moz_overlay = import (
    builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz
  );

  pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };

in
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustChannels.stable.rust-std
    rustChannels.stable.rust
    rustChannels.stable.rustc
    rustChannels.stable.cargo

    diesel-cli
    pgcli
    postgresql

    cmake
    curl
    gcc
    openssl
    pkgconfig
    which
    zlib
  ];

  LIBCLANG_PATH   = "${pkgs.llvmPackages.libclang}/lib";
}

