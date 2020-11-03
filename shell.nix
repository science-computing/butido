{ ... }:

let
  moz_overlay = import (
    builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz
  );

  pkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };

  pgcli-dev = pkgs.writeShellScriptBin "pgcli-dev" ''
    exec ${pkgs.pgcli}/bin/pgcli -h localhost -p 5432 ${PG_DB} ${PG_USER}
  '';

  PG_USER           = "pgdev";
  PG_DB             = "butido";
  PG_PW             = "password";
  PG_CONTAINER_NAME = "pg-dev-container";

in
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustChannels.stable.rust-std
    rustChannels.stable.rust
    rustChannels.stable.rustc
    rustChannels.stable.cargo

    diesel-cli
    pgcli
    pgcli-dev
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
  inherit PG_USER PG_DB PG_PW PG_CONTAINER_NAME;
}


