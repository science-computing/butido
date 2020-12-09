{ example ? "1", ... }:

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


  example_1_env = {
    BUTIDO_RELEASES     = "/tmp/example-1-releases";
    BUTIDO_STAGING      = "/tmp/example-1-staging";
    BUTIDO_SOURCE_CACHE = "/tmp/example-1-sources";
    BUTIDO_LOG_DIR      = "/tmp/example-1-logs";
    BUTIDO_REPO         = "/tmp/example-1-repo";

    BUTIDO_DATABASE_HOST     = "localhost";
    BUTIDO_DATABASE_PORT     = 5432;
    BUTIDO_DATABASE_USER     = PG_USER;
    BUTIDO_DATABASE_PASSWORD = PG_PW;
    BUTIDO_DATABASE_NAME     = PG_DB;
  };

  selectedEnv = {
    "1" = example_1_env;
  }."${example}";


in
pkgs.mkShell (
{
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

    devd # development web-server for serving sources locally.
  ];

  LIBCLANG_PATH   = "${pkgs.llvmPackages.libclang}/lib";
  inherit PG_USER PG_DB PG_PW PG_CONTAINER_NAME;
} // selectedEnv
)

