#!/usr/bin/env bash

die() {
    echo >&2 "$*"
    exit 1
}

[ -z "$PG_USER" ] && die "Not set: PG_USER"
[ -z "$PG_PW" ] && die "Not set: PG_PW"
[ -z "$PG_DB" ] && die "Not set: PG_DB"
[ -z "$PG_CONTAINER_NAME" ] && die "Not set: PG_CONTAINER_NAME"

docker run            \
    --name ${PG_CONTAINER_NAME}   \
    -e POSTGRES_PASSWORD=${PG_PW} \
    -p 5432:5432          \
    -m 512m           \
    -d                \
    --rm              \
    postgres

sleep 2
docker exec -it ${PG_CONTAINER_NAME} psql -U postgres -c "CREATE USER ${PG_USER} PASSWORD '${PG_PW}' SUPERUSER CREATEDB INHERIT LOGIN"
sleep 2
docker exec -it ${PG_CONTAINER_NAME} createdb -U postgres butido

echo "DONE"
